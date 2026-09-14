# 参考插画与混合动画素材 v1

日期：2026-09-13。分支：`anime-dev`。

用户选择了[橘猫参考图](references/orange-cat-user-reference.png)：大圆脸、眯眼笑、奶油色胸口、细前腿、小爪子和奶油色尾尖。新增插画帧序列作为默认预览，保留 SVG 分件绑定工作台；旧 SVG 后腿大腿横向收窄 36%，其余猫腿和爪子收窄 22%。两种素材目前可切换检查，尚未在同一角色动作内自动切换 renderer，也没有替换生产小屋/花园。

## 已实现的素材与动作

每个角色有 16 张透明 PNG 关键姿态、1 张无损 WebP atlas、7 个单次播放 GIF。猫和金毛共 32 张 PNG、2 张 atlas、14 个 GIF。GIF 用来下载/查看局部姿态；页面使用 atlas 和独立时间轴控制，包含场景位移。GIF 本身不包含猫窝、花圃或场景路径。

- [橘猫坐姿](../../../apps/desktop/src/assets/companion/sprites/v1/orange-cat/frames/00.png)、[蜷睡](../../../apps/desktop/src/assets/companion/sprites/v1/orange-cat/frames/14.png)、[睡觉 GIF](../../../apps/desktop/src/assets/companion/sprites/v1/orange-cat/previews/sleep.gif)
- [金毛坐姿](../../../apps/desktop/src/assets/companion/sprites/v1/golden-dog/frames/00.png)、[蜷睡](../../../apps/desktop/src/assets/companion/sprites/v1/golden-dog/frames/14.png)、[睡觉 GIF](../../../apps/desktop/src/assets/companion/sprites/v1/golden-dog/previews/sleep.gif)
- [帧时序与场景路径](../../../apps/desktop/src/assets/companion/sprites/v1/motions.json)
- [纯帧采样器](../../../apps/desktop/src/domain/companion/puppet/sprite.ts)

| 动作 | 时长 | 内容 |
| --- | --- | --- |
| 坐着 | 1.0 s | 参考坐姿，默认静止 |
| 起身 | 1.1 s | 坐姿、前倾、抬后腿、站立 |
| 走路 | 4.0 s | 起身、5 个四帧步态周期、站稳 |
| 跳上猫窝 | 2.2 s | 蓄力、伸展腾空、收腿、落地、站稳 |
| 睡觉 | 2.6 s | 坐姿、趴下、蜷睡、一次呼吸 |
| 坐下 | 1.1 s | 起身关键姿态倒序 |
| 醒来 | 1.3 s | 蜷睡、抬头、坐起 |
| 去花园 | 5.4 s | 起身、走到花圃旁、坐下 |

帧切换有 45 ms 短交叉淡化，位置按独立 route 插值。纯采样器不拥有计时器；页面在用户点击播放后运行有限 rAF，完成、暂停、窗口失焦、页面隐藏时取消。系统减少动态时禁用播放，保留静态拖动检查。选择另一个动作会回到该动作起点；尚未实现任意当前姿态到任意目标动作的连续状态机。

## 资产来源与导出

两张源图使用内置 Image Gen，根据用户参考生成；不是人工逐帧绘制，也不是 Spine 导出。生成要求及来源见 [generation-prompts.md](generation-prompts.md)。原始结果保存在各角色的 `source-sheet.png`。原始图片实际为 1254 × 1254 RGB；猫的生成结果把棋盘格画进了背景，金毛使用绿色底。

`scripts/companion/export-sprites.py` 不重画角色，只进行导出处理：背景色键清理、去除相邻格溢出的孤立墨点、按原像素比例居中和对齐脚底、生成 RGBA PNG / WebP / 有限 GIF。统一画布为 384 × 384，脚底基线 y=352；每个角色的 `manifest.json` 记录来源裁切框和偏移。

```sh
# 在仓库根目录启动独立预览；1420 留给 pnpm dev，1421 留给 Tauri HMR。
corepack pnpm --filter @jueming/desktop dev --host 127.0.0.1 --port 1422 --strictPort
# 导出脚本需要 Pillow 和 NumPy。
python3 scripts/companion/export-sprites.py
node tests/agent-companion/sprite-assets-contract.mjs
python3 scripts/companion/export-parts.py --check
node tests/agent-companion/puppet-assets-contract.mjs
corepack pnpm typecheck
corepack pnpm build
```

打开 `http://127.0.0.1:1422/companion-lab.html`。默认是插画动画；`SVG 绑定` 查看原分件动作，`全部切片` 检查/下载身体、四肢、眼睛、嘴巴、尾巴等独立 SVG。SVG 与插画是两套美术资源，不能把插画帧宣称为可编辑的 SVG 身体切片。

## 当前边界

这是 16 张关键姿态组成的动作素材原型。生成帧仍存在头部大小、视角和左右脚接触细节的变化；45 ms 淡化不能代替补间绘制、光流或严格足底约束。生产接入前需要补画走路/跳跃中间帧，统一转身视角，并把片段接到现有 Companion Controller 的取消、释放、编辑草稿和后台策略。小窝和花圃仍是开发验收场景。

目前已通过：32 张 PNG 的 RGBA/尺寸合同、14 个 GIF 无循环扩展、所有动作时长/边界/终点检查；SVG 绑定与导出检查；TypeScript 类型检查和生产构建。应用内浏览器已截图检查参考坐姿、走路中间帧、猫窝终点、猫/金毛蜷睡和花园终点，并实际播放验证睡觉结束后停止。未声称逐帧美术已达到最终生产质量。
