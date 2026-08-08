# miniserve 图片浏览功能实现方案

## 目标
为 miniserve 增加「类 Windows 文件管理器」式的图片浏览体验，保持项目「极简单二进制」的轻量定位（不引入图片处理依赖）。

## 设计决策（已与用户确认）
- **缩略图**：浏览器原生缩放（`<img>` 加载原图 + CSS `object-fit: cover` 缩放），**不引入** `image` crate
- **懒加载**：`<img loading="lazy">` + Intersection Observer 回退，图片进入视口才加载
- **图片识别**：按文件扩展名判断（`.jpg/.jpeg/.png/.gif/.webp/.svg/.bmp/.avif/.ico`）
- **视图模式**：3 种，顶部按钮切换 —— 列表（现有）、网格/缩略图、相册/平铺
- **左侧目录树**：要做（懒加载子树，点击在右侧加载内容）
- **点击图片**：lightbox 弹窗预览（支持左右切换、ESC 关闭）
- **状态持久化**：视图模式用 `localStorage` 记忆；URL query `?view=` 同步（可分享/刷新保持）

---

## 改动文件清单

### 1. `src/listing.rs`（数据层）
- **`Entry` 结构体**新增字段：`pub is_image: bool`（按扩展名判断）
  - 相应更新 `Entry::new()` 签名。
- **图片扩展名判断**：新增辅助函数 `is_image_by_extension(name)`。
- **`directory_listing`** 循环内构造 `Entry` 时填充 `is_image`。
- **`ListingQueryParameters`** 新增可选 `view` 参数 + 定义 `ViewMode` 枚举（`List` / `Grid` / `Album`）。

### 2. `src/renderer.rs`（渲染层 —— 改动核心）
- **`page()` 函数**：顶部工具栏后插入视图切换器；布局改为 `.workspace`（左树 + 右内容）；按 `view` 分支调用 `render_list_view()` / `render_grid_view()` / `render_album_view()`。
- 保留 `entry_row()` 用于列表视图；新增 `entry_card()`（网格）和 `entry_tile()`（相册）。
- 注入 lightbox 模态框容器。
- 内嵌 JS 扩展：目录树懒加载、视图切换、lightbox 交互。

### 3. `src/main.rs`（后端路由）
- 扩展 `ApiCommand` 枚举新增 `ListDirs { path }`，返回指定目录下子目录的 JSON。

### 4. `src/config.rs` / `src/args.rs`（配置层）
- 可选新增 CLI 参数 `--default-view <list|grid|album>`。
- `parametrized_link()` 扩展带上当前 `view` 参数。

### 5. `data/style.scss`（样式）
- 新增 `.workspace`/`.tree-sidebar`/`.content-area` 布局；`.view-switcher`；`.grid-view`/`.album-view`；懒加载占位；`.lightbox-modal`；目录树样式。

### 6. 测试
- `is_image_by_extension` 单元测试；`?view=` 视图渲染集成测试；`ListDirs` API 测试。

---

## 实施顺序（每步可独立编译验证）
1. 保存计划文档 ✅
2. 数据层：listing.rs 加 `is_image` + `ViewMode` + `?view=` 解析。
3. 后端 API：main.rs 扩展 `ApiCommand::ListDirs`。
4. 渲染层-列表视图：重构 `page()` 引入视图分支（list 视图行为不变）。
5. 渲染层-网格/相册视图：新增 `entry_card()` / `entry_tile()` + 视图切换器 UI。
6. 样式：style.scss 新增网格/相册/切换器样式。
7. JS：视图记忆 + lightbox + 目录树懒加载。
8. 样式：目录树 + lightbox + 侧边栏布局。
9. 可选 CLI：`--default-view` 参数。
10. 测试：单元测试 + 集成测试。
11. `cargo fmt && cargo clippy && cargo test` 全量验证。

## 风险与取舍说明
- **原图缩略图的流量代价**：网格视图会触发浏览器加载原图。懒加载缓解首屏，图片多/大时仍较重。这是「不引入图片处理依赖」的权衡。预留 `<img data-src>` 结构便于未来切换到服务端缩略图。
- **目录树改动较大**：需新增 JSON 端点 + 较多 JS。复用现有 api 路由机制，避免引入前端框架。
- **保持服务端渲染架构**：所有交互用原生 JS 内嵌（与现有上传/主题脚本风格一致）。
