# hton 项目概览

hton 是面向低成本 VPS 的自托管栈：一个公网 HTTPS 站点，其下运行选定的
VPN/代理服务，并带有一个小型自托管控制平面。外部只能看到普通网站；服务通过
TLS/SNI 和秘密路径区分。

## 状态

最小安装器已实现；完整的 R1 管理仍未完成。默认安装器把当前源码树部署到
`/opt/hton`，准备受信任的站点，并在获得明确本地同意后安全地重映射符合条件的
既有 Amnezia Xray。

## 快速开始

```sh
./scripts/hton/help
./scripts/hton/install
./scripts/hton/diag
./scripts/hton/share
```

## 文档

- [项目根 README](../../../README.md)
- [文档索引](../../README.md)
- [架构](../../ARCHITECTURE.md)
- [公开 CLI](../../CLI.md)
- [最小安装](../../MINIMUM-INSTALL.md)
- [用户脚本页面（英文）](../../scripts/)

英文是默认文档语言；本页为中文概览。
