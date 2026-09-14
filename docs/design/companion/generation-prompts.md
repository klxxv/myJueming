# 插画帧生成记录

工具：内置 Image Gen，2026-09-13。输入为用户提供的[参考图](references/orange-cat-user-reference.png)。以下为实际使用的两组提示词。两张生成结果都复制入仓库，导出脚本另行处理透明底、网格污染与脚底对齐；没有用旧 SVG 冒充参考插画。

## 橘猫

输出：`apps/desktop/src/assets/companion/sprites/v1/orange-cat/source-sheet.png`。

```text
Use case: illustration-story. Create ONE production sprite sheet for the EXACT orange tabby kitten shown in the supplied reference image. The reference is the character identity and art style reference, not a background to reproduce. Preserve its charming big round slightly tilted head, small triangular ears with peach interiors, closed crescent smiling eyes, tiny W smile, thin dark-brown hand-inked contour, warm golden-orange fur and darker soft tabby stripes, ivory muzzle and wide cream chest, tiny slender front legs and little ivory toes, up-curled tail with ivory tip. It must look like this specific kitten, not a generic thick-legged cartoon. Subtle painterly shading exactly like reference. Genuine transparent alpha background; no blue scene, ground, shadows, heart, text, labels, guides, borders or checkerboard drawn into the image.
Output square 2048x2048, EXACT regular 4-column by 4-row grid, each cell 512x512, 16 full-body frames. Every kitten is fully inside its cell with 45px padding, centered around x256, same head size and same paw baseline y460. Consistent character identity, colors, line weight, proportions and camera angle across every frame. No cell should have two cats. Frame order left-to-right, then top-to-bottom:
Row 1: (1) seated smiling, matching reference seated pose exactly, body slightly turned right and face toward viewer; (2) beginning to lean forward to rise, eyes gently open; (3) half risen, slender forepaws forward, hindquarters lifting; (4) standing on all four paws facing right in three-quarter view, face recognizable, SMALL slender legs.
Row 2: four consecutive coherent right-facing WALK cycle keyframes at same in-place position, standing body proportions exactly frame4: (5) near forepaw forward/far forepaw back; (6) legs passing under body; (7) opposite contact near forepaw back/far forepaw forward; (8) opposite passing pose. Keep paws small and legs slender.
Row 3: (9) low crouch ready to jump; (10) leap with front paws reaching forward and hind paws extended behind; (11) flight with paws gently tucked; (12) landing crouch with front paws touching baseline. Keep full body within cell; no route translation in sheet.
Row 4: (13) the same seated smiling pose as frame1; (14) gently bowing head and folding forepaws to lie down; (15) fully curled sleeping, eyes closed, cheek resting on front paws, tail wrapped alongside body; (16) the exact same sleeping silhouette and position as frame15 with a tiny breathing expansion only.
This is an actual frame atlas for a desktop pet, not a poster or presentation. Maintain regular grid and consistent scale. Faithfully match the supplied cat's appealing drawn quality.
```

## 金毛

输出：`apps/desktop/src/assets/companion/sprites/v1/golden-dog/source-sheet.png`。

```text
Use case illustration-story. Asset type: one 4x4 animation sprite atlas for a GOLDEN RETRIEVER PUPPY, the companion of the orange kitten in the supplied reference. Reference is STYLE only: same thin dark brown hand inked contour, warm soft painterly fill, sweet smiling expression, large rounded head and tiny slender legs, cream chest and ivory paws. Dog must clearly be a golden retriever, with soft long floppy golden ears, a cream longer muzzle, a dark little dog nose, a friendly tongue when awake, fluffy golden chest and a natural curved feathered tail. Not a cat, not flame tail, not thick mechanical legs. Honey gold fur, lighter cream muzzle and chest. Cute storybook rendering closely matching the reference.
Production sheet 2048x2048. EXACT 4 equal columns x 4 equal rows, sixteen separated full-body frames, ordered left-to-right top-to-bottom. All animals completely within respective cell with generous margin including tail. All cells consistent scale, camera, head size and paw baseline. No borders, text, heart, scene, shadows or labels. Background MUST be uniform flat pure chroma green #00FF00 across entire image, no checkerboard, no gradient, no glow; keep green absent from puppy.
Row1: 1 seated smiling puppy body slightly turned right and face toward viewer; 2 leaning forward to rise; 3 half risen with hindquarters lifting; 4 standing on four SMALL slender legs facing right in three-quarter view.
Row2: 5 walk right near forepaw forward; 6 walking passing position forepaws under body; 7 opposite walk contact near forepaw back and far forward; 8 opposite passing position. In place, same character scale and position, coherent contact leg order.
Row3: 9 crouched ready to jump; 10 leaping forward forepaws reaching right hindlegs trailing; 11 flight with legs softly tucked; 12 landing crouch front paws touching ground. No camera movement, route displacement or scene.
Row4: 13 same seated smiling pose as1; 14 folding forepaws and gently bowing head to lie down; 15 sleeping curled, eyes closed, MOUTH CLOSED, cheek on paws, tail resting beside body; 16 identical sleeping pose and position with a tiny breathing expansion only.
This is a real animation atlas, not a decorative collage. Match the charming reference illustration quality.
```
