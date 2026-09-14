# 桌宠 SVG 分件素材与动作包 v1

日期：2026-09-13。分支：`anime-dev`。

后续用户选择参考插画，已新增 [插画序列与混合动画包](sprite-hybrid-v1.md)。本文描述保留的 SVG 分件通道，默认预览已切换为插画帧动画。

本轮交付用于美术、绑定与动作开发：胖橘猫和金毛的原创分层 SVG、独立透明切片、关节清单、独立动作数据，以及可播放、逐帧拖动的开发预览。风格沿用现有橘猫和金毛的暖色插画，新增朝右的四足姿态。

这是分件动画素材准备阶段。当前生产 `CompanionHabitat`、`CompanionGarden` 与 `CompanionRenderer` 尚未切换到这套素材；预览中的小窝和花园为动作验收场景，不代表已经实现应用内跨区域行走。没有安装 Pixi 或 Spine，也不是 Spine 导出格式。

## 资源入口

- [橘猫母版](../../../apps/desktop/src/assets/companion/puppets/v1/orange-cat/master.svg) / [部件总览](../../../apps/desktop/src/assets/companion/puppets/v1/orange-cat/parts-sheet.svg) / [绑定清单](../../../apps/desktop/src/assets/companion/puppets/v1/orange-cat/rig.json)
- [金毛母版](../../../apps/desktop/src/assets/companion/puppets/v1/golden-dog/master.svg) / [部件总览](../../../apps/desktop/src/assets/companion/puppets/v1/golden-dog/parts-sheet.svg) / [绑定清单](../../../apps/desktop/src/assets/companion/puppets/v1/golden-dog/rig.json)
- [独立动作数据](../../../apps/desktop/src/assets/companion/puppets/v1/motions.json)
- [动作数据类型](../../../apps/desktop/src/domain/companion/puppet/types.ts) / [无计时器采样器](../../../apps/desktop/src/domain/companion/puppet/sample.ts)

## 分件约定

橘猫 31 片、28 个关节；金毛 29 片、26 个关节。多出的眼睛和嘴巴切片是同一关节的替换表情，不是额外骨骼。

| 分组 | 独立切片 |
| --- | --- |
| 身体 | body、bib（胸前绒毛） |
| 四肢 | front/hind × near/far × upper/lower/paw，共 12 片；前肢为上肢、下肢、爪子，后肢为大腿、小腿、爪子 |
| 尾巴 | tail-base、tail-mid、tail-tip，三级父子关节 |
| 头部 | head、ear-far、ear-near、muzzle、nose |
| 表情 | 双眼各有 open/closed；mouth-smile、mouth-open、tongue |
| 橘猫额外 | whiskers-far、whiskers-near |

`near/far` 指相对于观察者的近侧、远侧。当前是一套朝右的固定视角；朝左由整个角色根节点镜像。它不是完整多视角转身素材。

1. `master.svg` 是美术源文件。每个 `g[data-part]` 对应一个切片，保留可读路径；母版不包含动画、脚本或位图。替换表情在母版中 `display="none"`，导出时仍生成独立文件。
2. `rig.json` 是绑定源文件。`bones` 按父级先于子级排列，`parts` 按远到近的绘制顺序排列，两者相互独立。
3. 切片使用裁切后的 `viewBox`，可以有负坐标；每片的关节原点始终是局部 `(0, 0)`，不把几何中心错误地当旋转中心。`bounds` 与切片 `viewBox` 一致。
4. 父关节的局部平移来自 `bone.x/y`。动作中的平移、角度都是相对于静止绑定的增量；缩放默认 1。先做局部变换，再乘父级矩阵。
5. 母版画布为 `320 × 280`，绘制时根位置为 `origin`；预览场景使用自己的角色根位置。`groundY` 记录站立时足底参考线。
6. 各部件保留关节重叠区域以便旋转；四肢的上端接缝不画完整轮廓线。远侧肢体先于身体绘制，近侧肢体后绘制。
7. `awake`、`sleep`、`happy` 控制表情切片替换。未声明 `expressions` 的部件始终显示。

## 动作与路径分离

`motions.json` 包含两层数据，均不引用 SVG 路径或纹理页：

- `clips`：只描述局部关节和表情。当前有 rest、walk、jump、lie-down、sleep、look 六个片段。
- `actions`：按有限时长组合片段，并用独立 `route` 指定角色在场景中的位移。换场景时可以替换路径而复用肢体动作。

| 动作 | 片段 | 时长 | 最终状态 |
| --- | --- | --- | --- |
| 站立 | rest | 1.0 s | 默认站姿 |
| 走路 | walk × 4 | 3.6 s | 前进 235 单位后站立 |
| 跳上猫窝 | jump | 1.7 s | 蓄力、腾空、落地；前进 220、抬升 80 单位 |
| 睡觉 | lie-down → sleep | 3.8 s | 收腿、闭眼、一次呼吸后保持睡姿 |
| 去花园 | walk × 4 → look | 5.4 s | 前进 340 单位，张望、摇尾后站立 |

时间轴 key 的 `at` 为 0–1；同一通道线性插值，未声明通道采用中性值。每个动作的 stages 连续覆盖时长，`cycles` 必须为正整数。终点保持最后姿态，不自动回到首帧。重复步态的首尾关节姿态相同。动作完成不触发 canonical 写入。

当前四足步态为绑定与素材验收用原型，尚未做逐脚接触约束、IK、不同体型步幅校准、复杂碰撞或猫窝遮挡蒙版。生产接入前应继续打磨足底滑动、尾部接缝和落地缓冲；不要把这一版标记为成品骨骼动画。

## 开发预览与导出

仓库根目录运行：

```sh
corepack pnpm --filter @jueming/desktop dev --host 127.0.0.1 --port 1422 --strictPort
```

打开 `http://127.0.0.1:1422/companion-lab.html`。这是独立 Vite 开发入口，不在主应用导航中，不进入默认生产入口。

- 选择橘猫或金毛、拼装预览或全部切片。
- 点击部件查看局部锚点与父级信息，可下载单片 SVG。
- 选择动作，播放一次；可暂停、复位、调速、镜像或拖动进度检查姿态。
- 显示关节辅助线，检查父子连接和遮挡。
- 默认不自动播放；系统减少动态时只允许静态拖动；页面隐藏或窗口失焦立即暂停，不自动恢复。

编辑 `master.svg` 的部件内部路径后重新导出：

```sh
python3 scripts/companion/export-parts.py
python3 scripts/companion/export-parts.py --check
node tests/agent-companion/puppet-assets-contract.mjs
corepack pnpm typecheck
corepack pnpm build
```

`parts/*.svg` 和 `parts-sheet.svg` 由导出脚本派生，不直接编辑。改变关节位置时同时更新 `rig.json` 和母版对应组的静止 `transform`。切片边界要容纳描边；不要优化掉 `data-part`、`data-bone` 或变换原点。文件均为本仓库原创矢量源，未引用外部角色、字体或远程资源。

浏览器回归入口为 `tests/agent-companion/puppet-lab-browser.mjs`，需先启动上述 Vite 开发服务器，并使用本机已有 Playwright；若它不在默认模块搜索路径中，将 `PLAYWRIGHT_MODULE` 指向其 `index.mjs`。截图写入已忽略的 `test-output/companion-lab/`。

初版验证曾通过：60 片 SVG 的实际几何边界、两只角色的四个动作及终点、切片下载、关节显示和镜像、有限播放停止、失焦暂停、减少动态、护眼主题，以及 900/390 像素布局。类型检查、生产构建、绑定/动作合同检查通过。生产构建仍不包含此开发入口和新素材包。本次参考插画迭代的验证范围见后续文档。初版浏览器验证使用 Chromium；原生 Windows WebView2/macOS WKWebView 验收留到生产 renderer 接入阶段。

## 后续生产接入

在现有 `CompanionRenderer` 后增加素材适配器，继续由 Controller 管理懒加载、取消和释放。共享角色实例状态负责小屋/花园锚点，局部关节动作与场景位移分别采样。需要 Pixi 时，可将切片离线栅格化并生成 atlas；需要 Spine 时，仍须在 Spine 工具链中制作/导出其专用绑定与动画数据。

生产动作仍要服从静态、安静、隐藏、系统减少动态、后台与编辑草稿策略；主平行视图、批准按钮和项目提交不等待动画。当前开发预览所用 `requestAnimationFrame` 只在用户启动的有限动作内运行，停止后没有常驻 ticker。
