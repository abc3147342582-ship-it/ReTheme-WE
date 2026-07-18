# ReTheme 主题开发规范

ReTheme 主题是声明式资源包：`manifest.json`、CSS 和图片。主题不能包含 JavaScript，也不直接使用 ChatGPT 的内部类名或选择器；运行时把稳定插槽映射到当前 ChatGPT 版本。

## 目录结构

```text
my-theme/
├── manifest.json
├── styles/
│   ├── tokens.css
│   └── overrides.css
└── assets/
    └── hero.svg
```

本地开发直接选择该目录。社区投稿上传 ZIP，ZIP 根目录必须直接包含 `manifest.json`。

## 最小 Manifest

可运行示例见 [`docs/theme-example/manifest.json`](theme-example/manifest.json)。关键字段：

| 字段 | 规则 |
|:--|:--|
| `schemaVersion` | 当前固定为 `1` |
| `id` | 全局唯一，建议反向域名格式，发布后不可变 |
| `version` | 合法 SemVer；已发布版本不可覆盖 |
| `styles` | 包内 CSS 相对路径，至少一个 |
| `slots` | CSS 使用的稳定逻辑插槽 |
| `permissions` | 当前必须为 `[]` |
| `preview` | 商店与客户端使用的三个十六进制预览色 |
| `experience` | 首页 Banner、会话窄 Banner、提示文案和受控装饰资源 |
| `locales` | 可选本地化；当前建议完整提供 `en`，基础字段作为中文默认值 |

`testedCodexVersions` 只记录实际回归过的 ChatGPT 版本，不会锁定运行版本。平台兼容数据负责为不同版本选择匹配规则。

## CSS 作用域

所有选择器必须位于主题根作用域下：

```css
:root[data-ct-theme="studio.example.protocol-preview"] [data-ct-slot="sidebar"] {
  background: var(--rt-sidebar-background) !important;
}
```

不要使用 ChatGPT 的压缩类名、DOM 层级或文本内容作为选择器。不要覆盖字体；ReTheme 保留 ChatGPT 的原生字体，并在 Windows 自身界面优先使用 Microsoft YaHei。

建议将颜色、圆角、边框、阴影和间距放在 `tokens.css`，把插槽规则放在 `overrides.css`。明暗主题可使用：

```css
:root[data-ct-theme="studio.example.protocol-preview"][data-ct-color-scheme="dark"] {
  --rt-page-background: #101521;
}
```

## 常用插槽

| 区域 | 常用插槽 |
|:--|:--|
| 应用 | `app.shell`, `app.background`, `titlebar` |
| 侧栏 | `sidebar`, `sidebar.header`, `sidebar.item`, `sidebar.item.active`, `sidebar.footer` |
| 主内容 | `main`, `main.background`, `page`, `page.surface`, `page.header` |
| 首页 | `home.hero`, `home.prompt.title`, `home.cards`, `home.card`, `home.card.background`, `home.card.label` |
| 会话 | `conversation.header`, `conversation.banner`, `conversation`, `conversation.user`, `conversation.assistant` |
| 输入框 | `composer`, `composer.context`, `composer.editor`, `composer.action`, `composer.submit` |
| 菜单 | `menu`, `menu.item`, `menu.item.active`, `menu.separator` |
| 设置 | `settings`, `settings.canvas`, `settings.card`, `settings.row`, `settings.switch.track.checked` |

只在 `slots` 中声明实际使用的插槽。完整白名单以客户端 `src-tauri/src/theme.rs` 的 `ALLOWED_SLOTS` 为准；主题不得自行创建新的 `data-ct-slot`。

## Banner 与静态资源

首页 Banner 和会话窄 Banner 分开声明。`asset` 是背景，`foreground` 是可选透明人物或物品层，底层引擎负责稳定挂载与布局：

```json
{
  "homeHero": {
    "eyebrow": "RETHEME",
    "title": "创造自己的工作氛围",
    "description": "稳定插槽驱动的示例主题。",
    "asset": "assets/hero.svg",
    "fit": "cover",
    "position": "center"
  },
  "conversationBanner": {
    "eyebrow": "RETHEME",
    "title": "保持专注",
    "description": "让当前会话保持清晰。",
    "asset": "assets/hero.svg",
    "fit": "cover",
    "position": "center"
  }
}
```

允许 PNG、JPEG、WebP 和经过消毒的静态 SVG。单图最大 8 MiB，压缩包最大 30 MiB，解压后最大 60 MiB，文件数最大 256。主题资源由 ReTheme 本地回环静态服务提供，不需要 Base64，也不得加载外部 URL、`@import` 或远程字体。

动画只使用 `transform` 和 `opacity`，并支持减少动态效果：

```css
@media (prefers-reduced-motion: reduce) {
  :root[data-ct-theme="studio.example.protocol-preview"] * {
    animation: none !important;
  }
}
```

## 双语

主题默认字段使用中文，`locales.en` 覆盖名称、说明、首页文案、首页提示和会话 Banner。ReTheme 会把自身界面语言同步给运行中的主题，无需根据 ChatGPT 文案判断语言。

## 开发与发布

1. 复制 [`docs/theme-example`](theme-example) 并修改 `id`。
2. 在 ReTheme 中选择“加载本地主题”。本地目录无需签名、加密或 `.ctheme`。
3. 同时验证中文/英文、浅色/深色、首页/会话/设置、不同窗口宽度。
4. 删除无用资源，更新 SemVer，将源码目录压缩为 ZIP。
5. 在 ReTheme 社区投稿 ZIP。不要自行添加 `access`、`integrity` 或 `signature`。
6. 平台审核后生成正式 `.ctheme`；普通用户只能通过在线下载安装。

正式包的签名与加密由平台完成。开发者私钥、平台密钥和 ChatGPT 适配选择器都不属于主题源码。
