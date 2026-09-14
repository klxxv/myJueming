# 桌宠动画现状与运行库调查

核对日期：2026-09-13。

## 当前代码

生产前端没有安装 Spine、PixiJS 或 sprite 动画库。`CompanionHabitat.vue` 和 `CompanionGarden.vue` 引用静态 `orange-cat.svg` / `golden-dog.svg`；两个位置展示猫和金毛，使用 Controller 控制活动/静态模式。`WebAnimationsCompanionRenderer` 对整个 DOM 角色做有限的平移、旋转和缩放。这里没有 Spine skeleton、atlas、mesh 或独立腿部绑定。

`anime-dev` 的开发预览增加了两个素材通道：`puppets/v1` 保存 SVG 分件、关节和局部动作；`sprites/v1` 保存插画关键帧、atlas、GIF 和帧时序。两者共用页面的有限播放/暂停控制，场景路径与局部动作分开。生产入口尚未导入新素材。

## 库比较

| 方案 | 核对的能力 | 对本项目的判断 |
| --- | --- | --- |
| PixiJS AnimatedSprite | 接受纹理序列或 Spritesheet，支持 gotoAndStop、播放/停止和 autoUpdate 控制 | 角色数量和特效增长后是合适的统一 2D renderer 候选；接入时关闭共享 ticker 自动更新，由 Controller 管生命周期。[官方 API](https://pixijs.download/release/docs/scene.AnimatedSprite.html) |
| Anime.js | 对目标属性做动画，提供 JS 版与 WAAPI 版 animate | 适合 UI、SVG 属性和场景路径补间；它本身不会生成猫的步态素材。[官方文档](https://animejs.com/documentation/animation/) |
| Phaser | 原生支持 Sprite 帧动画、时长、重复次数，也可用 Tween；配有场景和纹理系统 | 能完整承载小游戏，但当前只有少量桌宠，不需要引入完整游戏运行框架。[官方文档](https://docs.phaser.io/phaser/concepts/animations) |
| Spine | 专门的编辑器/运行库工作流与授权条款 | 适合后续精细骨骼/网格制作；当前 SVG rig JSON 不是 Spine 数据。正式接入前核对编辑器与运行库授权。[运行库条款](https://en.esotericsoftware.com/spine-runtimes-license) |
| 本轮的 SVG + atlas 采样器 | 原生 SVG 视口裁切 atlas，纯函数采样帧/关节，再由有限 rAF 播放 | 当前素材验证不增加运行依赖；GIF 作为单次播放的预览导出，页面直接控制帧序列。 |

建议来自当前项目规模和上述能力对比：先确定角色风格、姿态与片段边界；需要更多实例、纹理管理和渲染特效时，再将当前 atlas/动作数据适配到 Pixi。继续保留静态模式、减少动态、后台停止和 dirty-editor 约束。
