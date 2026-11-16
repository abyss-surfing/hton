//! Closed age/OpenSSL/ACME operations; credential-bearing bytes never enter ordinary logs.
use crate::{
    Budget, Cancellation, Error, Result,
    process::{Runner, Tool},
};
use hton_storage::SecureDir;
use std::{
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
};

pub struct ProtectedBytes(Vec<u8>);
impl ProtectedBytes {
    pub fn expose(&self) -> &[u8] {
        &self.0
    }
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }
}
impl fmt::Debug for ProtectedBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ProtectedBytes([redacted])")
    }
}
#[derive(Clone)]
pub struct AgeRecipient(String);
impl AgeRecipient {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if !value.starts_with("age1")
            || value.len() < 16
            || value.len() > 128
            || !value
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        {
            return Err(Error::InvalidInput);
        }
        Ok(Self(value))
    }
}
impl fmt::Debug for AgeRecipient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AgeRecipient([public-key])")
    }
}
#[derive(Clone)]
pub struct PrivateDirectory {
    path: PathBuf,
    directory: Arc<SecureDir>,
}
impl PrivateDirectory {
    pub fn open(path: &Path) -> Result<Self> {
        if !path.is_absolute() {
            return Err(Error::InvalidInput);
        }
        let directory = SecureDir::open_private(path).map_err(|_| Error::InvalidInput)?;
        Ok(Self {
            path: path.to_owned(),
            directory: Arc::new(directory),
        })
    }
    pub fn file(&self, name: &str, max: usize) -> Result<PrivateFile> {
        let bytes = self
            .directory
            .read(name, max)
            .map_err(|_| Error::InvalidInput)?
            .ok_or(Error::InvalidInput)?;
        Ok(PrivateFile {
            directory: self.clone(),
            name: name.to_owned(),
            bytes,
        })
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
}
impl fmt::Debug for PrivateDirectory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PrivateDirectory([capability])")
    }
}
#[derive(Clone)]
pub struct PrivateFile {
    directory: PrivateDirectory,
    name: String,
    bytes: Vec<u8>,
}
impl PrivateFile {
    fn path(&self) -> PathBuf {
        self.directory.path.join(&self.name)
    }
    fn verify(&self) -> Result<()> {
        if self
            .directory
            .directory
            .read(&self.name, self.bytes.len())
            .map_err(|_| Error::Io)?
            .as_deref()
            != Some(self.bytes.as_slice())
        {
            return Err(Error::IdentityMismatch);
        }
        Ok(())
    }
}
impl fmt::Debug for PrivateFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PrivateFile([protected])")
    }
}
#[derive(Clone)]
pub struct PrivateBlob {
    directory: PrivateDirectory,
    name: String,
    size: u64,
}
impl fmt::Debug for PrivateBlob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PrivateBlob([protected])")
    }
}
impl PrivateDirectory {
    pub fn blob(&self, name: &str, max_bytes: u64) -> Result<PrivateBlob> {
        let size = self
            .directory
            .file_size(name, max_bytes)
            .map_err(|_| Error::InvalidInput)?
            .ok_or(Error::InvalidInput)?;
        Ok(PrivateBlob {
            directory: self.clone(),
            name: name.to_owned(),
            size,
        })
    }
}
impl PrivateBlob {
    fn path(&self) -> PathBuf {
        self.directory.path.join(&self.name)
    }
    fn verify(&self) -> Result<()> {
        if self
            .directory
            .directory
            .file_size(&self.name, self.size)
            .map_err(|_| Error::IdentityMismatch)?
            != Some(self.size)
        {
            return Err(Error::IdentityMismatch);
        }
        Ok(())
    }
}
#[derive(Clone)]
pub struct CryptoTools {
    runner: Arc<Runner>,
}
impl CryptoTools {
    pub(crate) fn new(runner: Arc<Runner>) -> Self {
        Self { runner }
    }
    pub fn generate_age_identity(
        &self,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<ProtectedBytes> {
        let bytes = self
            .runner
            .run_input(Tool::AgeKeygen, &[], &[], budget, cancel)?;
        if bytes.is_empty() || bytes.len() > 16384 {
            return Err(Error::InvalidResponse);
        }
        let _ = self.derive_age_recipient(&bytes, budget, cancel)?;
        Ok(ProtectedBytes(bytes))
    }
    pub fn derive_age_recipient(
        &self,
        identity: &[u8],
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<AgeRecipient> {
        if identity.is_empty() || identity.len() > 16384 {
            return Err(Error::InvalidInput);
        }
        let bytes = self
            .runner
            .run_input(Tool::AgeKeygen, &["-y"], identity, budget, cancel)?;
        let text = std::str::from_utf8(&bytes)
            .map_err(|_| Error::InvalidResponse)?
            .trim();
        AgeRecipient::new(text)
    }
    pub fn export_image(
        &self,
        image: &crate::docker::ImageReference,
        output: &PrivateDirectory,
        name: &str,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<u64> {
        crate::docker::DockerObserver::new(self.runner.clone())
            .inspect_image(image, budget, cancel)?;
        if !name.ends_with(".tar") || name.len() > 128 {
            return Err(Error::InvalidInput);
        }
        let locked = output
            .directory
            .lock(std::time::Instant::now() + std::time::Duration::from_secs(1))
            .map_err(|_| Error::Io)?;
        if locked
            .read(name, 0)
            .map_err(|_| Error::InvalidInput)?
            .is_some()
        {
            return Err(Error::InvalidInput);
        }
        locked
            .compare_exchange(name, None, &[])
            .map_err(|_| Error::Io)?;
        let path = output.path.join(name);
        let path = path.to_str().ok_or(Error::InvalidInput)?;
        self.runner.run(
            Tool::Docker,
            &[
                "--host",
                "unix:///var/run/docker.sock",
                "--config",
                "/opt/hton/work/docker-client",
                "image",
                "save",
                "--output",
                path,
                image.as_str(),
            ],
            budget,
            cancel,
        )?;
        let size = output
            .directory
            .seal_external_file(name, 8 * 1024 * 1024 * 1024)
            .map_err(|_| Error::Io)?;
        if size == 0 {
            return Err(Error::InvalidResponse);
        }
        budget.check(cancel)?;
        Ok(size)
    }
    pub fn import_image(
        &self,
        image: &crate::docker::ImageReference,
        input: &PrivateBlob,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<()> {
        input.verify()?;
        let path = input.path();
        let path = path.to_str().ok_or(Error::InvalidInput)?;
        self.runner.run(
            Tool::Docker,
            &[
                "--host",
                "unix:///var/run/docker.sock",
                "--config",
                "/opt/hton/work/docker-client",
                "image",
                "load",
                "--input",
                path,
            ],
            budget,
            cancel,
        )?;
        input.verify()?;
        crate::docker::DockerObserver::new(self.runner.clone())
            .inspect_image(image, budget, cancel)?;
        budget.check(cancel)
    }
    pub fn age_encrypt(
        &self,
        recipients: &[AgeRecipient],
        plaintext: &[u8],
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<ProtectedBytes> {
        if recipients.is_empty() || recipients.len() > 32 {
            return Err(Error::InvalidInput);
        }
        let mut args = vec!["--encrypt"];
        for recipient in recipients {
            args.push("--recipient");
            args.push(&recipient.0);
        }
        let output = self
            .runner
            .run_input(Tool::Age, &args, plaintext, budget, cancel)?;
        if !output.starts_with(b"age-encryption.org/v1\n") {
            return Err(Error::InvalidResponse);
        }
        budget.check(cancel)?;
        Ok(ProtectedBytes(output))
    }
    pub fn age_decrypt(
        &self,
        identity: &PrivateFile,
        ciphertext: &[u8],
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<ProtectedBytes> {
        identity.verify()?;
        let path = identity.path();
        let path = path.to_str().ok_or(Error::InvalidInput)?;
        let output = self.runner.run_input(
            Tool::Age,
            &["--decrypt", "--identity", path],
            ciphertext,
            budget,
            cancel,
        )?;
        identity.verify()?;
        budget.check(cancel)?;
        Ok(ProtectedBytes(output))
    }
    pub fn certificate_der(
        &self,
        pem: &[u8],
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<ProtectedBytes> {
        self.openssl_input(&["x509", "-outform", "DER"], pem, budget, cancel)
    }
    pub fn certificate_public_key(
        &self,
        pem: &[u8],
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<ProtectedBytes> {
        let public = self.openssl_input(&["x509", "-pubkey", "-noout"], pem, budget, cancel)?;
        self.openssl_input(
            &["pkey", "-pubin", "-outform", "DER"],
            public.expose(),
            budget,
            cancel,
        )
    }
    pub fn private_public_key(
        &self,
        pem: &[u8],
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<ProtectedBytes> {
        self.openssl_input(
            &["pkey", "-pubout", "-outform", "DER", "-passin", "pass:"],
            pem,
            budget,
            cancel,
        )
    }
    pub fn certificate_metadata(
        &self,
        pem: &[u8],
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<ProtectedBytes> {
        self.openssl_input(
            &["x509", "-noout", "-dates", "-ext", "subjectAltName"],
            pem,
            budget,
            cancel,
        )
    }
    fn openssl_input(
        &self,
        args: &[&str],
        input: &[u8],
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<ProtectedBytes> {
        let bytes = self
            .runner
            .run_input(Tool::OpenSsl, args, input, budget, cancel)?;
        budget.check(cancel)?;
        Ok(ProtectedBytes(bytes))
    }
    pub fn verify_chain(
        &self,
        pem: &[u8],
        trust: &PrivateFile,
        now_seconds: u64,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<()> {
        use std::{io::Write, os::fd::AsRawFd};
        trust.verify()?;
        // Public chain only: anonymous sealed memory, no disk staging in read-only checks.
        let fd = rustix::fs::memfd_create(
            "hton-certificate-chain",
            rustix::fs::MemfdFlags::ALLOW_SEALING,
        )
        .map_err(|_| Error::Io)?;
        let mut chain = std::fs::File::from(fd);
        chain.write_all(pem).map_err(|_| Error::Io)?;
        rustix::fs::fcntl_add_seals(
            &chain,
            rustix::fs::SealFlags::SEAL
                | rustix::fs::SealFlags::SHRINK
                | rustix::fs::SealFlags::GROW
                | rustix::fs::SealFlags::WRITE,
        )
        .map_err(|_| Error::Io)?;
        let chain_path = format!("/proc/self/fd/{}", chain.as_raw_fd());
        let trust_path = trust.path();
        let now = now_seconds.to_string();
        self.runner.run_input(
            Tool::OpenSsl,
            &[
                "verify",
                "-purpose",
                "sslserver",
                "-attime",
                &now,
                "-no-CApath",
                "-no-CAstore",
                "-CAfile",
                trust_path.to_str().ok_or(Error::InvalidInput)?,
                "-untrusted",
                &chain_path,
            ],
            pem,
            budget,
            cancel,
        )?;
        trust.verify()?;
        budget.check(cancel)
    }
    pub fn issue_acme(
        &self,
        image: &crate::docker::ImageReference,
        work: &PrivateDirectory,
        credentials: &PrivateFile,
        request_component: &str,
        names: &[String],
        staging: bool,
        email: &str,
        container_name: &str,
        on_created: &dyn Fn(&crate::docker::ContainerId) -> Result<()>,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<()> {
        if names.is_empty()
            || names.len() > 100
            || email.len() > 254
            || !email.bytes().all(|b| b.is_ascii_graphic())
            || !email.contains('@')
            || container_name.len() > 128
            || !container_name.starts_with("hton-acme-")
            || !container_name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return Err(Error::InvalidInput);
        }
        for name in names {
            let host = name.strip_prefix("*.").unwrap_or(name);
            if host.is_empty()
                || host.len() > 253
                || host.split('.').any(|label| {
                    label.is_empty()
                        || label.len() > 63
                        || label.starts_with('-')
                        || label.ends_with('-')
                        || !label
                            .bytes()
                            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
                })
            {
                return Err(Error::InvalidInput);
            }
        }
        if !request_component.starts_with("r-")
            || request_component.len() > 128
            || !request_component
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(Error::InvalidInput);
        }
        credentials.verify()?;
        let fullchain = format!("/acme.sh/{request_component}/output/fullchain.pem");
        let privatekey = format!("/acme.sh/{request_component}/output/key.pem");
        let mount = format!(
            "type=bind,source={},target=/acme.sh",
            work.path.to_str().ok_or(Error::InvalidInput)?
        );
        let envpath = credentials.path();
        let envpath = envpath.to_str().ok_or(Error::InvalidInput)?;
        let server = if staging {
            "https://acme-staging-v02.api.letsencrypt.org/directory"
        } else {
            "https://acme-v02.api.letsencrypt.org/directory"
        };
        let mut args = vec![
            "--host",
            "unix:///var/run/docker.sock",
            "--config",
            "/opt/hton/work/docker-client",
            "create",
            "--pull",
            "never",
            "--name",
            container_name,
            "--label",
            "io.hton.role=acme",
            "--log-driver",
            "none",
            "--memory",
            "128m",
            "--cpus",
            "0.5",
            "--pids-limit",
            "64",
            "--read-only",
            "--cap-drop",
            "ALL",
            "--security-opt",
            "no-new-privileges:true",
            "--user",
            "0:0",
            "--tmpfs",
            "/tmp:rw,noexec,nosuid,size=16777216",
            "--env",
            "AUTO_UPGRADE=0",
            "--env-file",
            envpath,
            "--mount",
            &mount,
            "--entrypoint",
            "/usr/local/bin/acme.sh",
            image.as_str(),
            "--issue",
            "--home",
            "/acmebin",
            "--config-home",
            "/acme.sh",
            "--cert-home",
            "/acme.sh/certs",
            "--dns",
            "dns_internetbs",
            "--server",
            server,
            "--accountemail",
            email,
            "--keylength",
            "ec-256",
            "--dnssleep",
            "120",
            "--fullchain-file",
            &fullchain,
            "--key-file",
            &privatekey,
        ];
        for name in names {
            args.push("-d");
            args.push(name);
        }
        let created = self.runner.run(Tool::Docker, &args, budget, cancel)?;
        let id = std::str::from_utf8(&created)
            .map_err(|_| Error::InvalidResponse)?
            .trim();
        let id = crate::docker::ContainerId::new(id)?;
        // Persist the exact owned job identity before external ACME/DNS work starts.
        let checkpoint = on_created(&id);
        let started = checkpoint.and_then(|()| {
            self.runner.run(
                Tool::Docker,
                &[
                    "--host",
                    "unix:///var/run/docker.sock",
                    "--config",
                    "/opt/hton/work/docker-client",
                    "start",
                    "--attach",
                    id.as_str(),
                ],
                budget,
                cancel,
            )
        });
        let cleanup_budget = Budget::new(
            std::time::Instant::now() + std::time::Duration::from_secs(5),
            65536,
        )?;
        let cleanup_cancel = std::sync::atomic::AtomicBool::new(false);
        self.runner.run(
            Tool::Docker,
            &[
                "--host",
                "unix:///var/run/docker.sock",
                "--config",
                "/opt/hton/work/docker-client",
                "rm",
                "--force",
                id.as_str(),
            ],
            cleanup_budget,
            &cleanup_cancel,
        )?;
        started?;
        credentials.verify()?;
        budget.check(cancel)
    }
}
