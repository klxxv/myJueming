import type { AlignmentDto, SegmentDto } from "../domain/kernel-client";

export const governmentSource = [
  "过去一年，面对复杂严峻的外部环境和艰巨繁重的改革发展稳定任务，我们坚持稳中求进工作总基调，国民经济运行总体平稳、稳中有进。",
  "经济结构持续优化，新动能加快成长，创新驱动发展战略深入实施。",
  "就业物价总体平稳，居民收入稳步增长，民生保障有力有效。",
  "我们深入推进高质量发展，加快构建新发展格局，经济发展质量和效益不断提升。",
  "深化改革扩大开放，市场活力和社会创造力进一步激发。",
  "生态文明建设扎实推进，绿色低碳转型取得积极进展。",
  "政府自身建设不断加强，治理能力和服务水平持续提升。",
  "这些成绩来之不易，是全国各族人民团结奋斗的结果。",
] as const;

export const governmentTarget = [
  "Over the past year, in the face of a complex and severe external environment and arduous tasks in reform, development, and stability, we adhered to the general principle of pursuing progress while maintaining stability. The national economy remained broadly stable and moved forward steadily.",
  "The economic structure continued to improve, new growth drivers accelerated their development, and the innovation-driven development strategy was implemented in depth.",
  "Employment and prices remained generally stable, residents’ incomes grew steadily, and people’s well-being and social security were effectively strengthened.",
  "We advanced high-quality development in depth, accelerated the building of a new development paradigm, and continuously improved the quality and efficiency of economic growth.",
  "Reforms were deepened and opening up was expanded, further stimulating market vitality and social creativity.",
  "The building of ecological civilization was solidly advanced, and positive progress was made in the green and low-carbon transition.",
  "The Government strengthened its own building, and governance capacity and service levels continued to improve.",
  "These achievements are hard-won, achieved through the joint efforts of people of all ethnic groups across the country.",
] as const;

export const sourceSegments: SegmentDto[] = governmentSource.map((text, index) => ({
  id: `src-${String(index + 1).padStart(6, "0")}`,
  side: "source",
  text,
  order: index,
}));

export const targetSegments: SegmentDto[] = governmentTarget.map((text, index) => ({
  id: `tgt-${String(index + 1).padStart(6, "0")}`,
  side: "target",
  text,
  order: index,
}));

export const alignments: AlignmentDto[] = sourceSegments.map((source, index) => ({
  id: `alignment-${String(index + 1).padStart(6, "0")}`,
  sourceIds: [source.id],
  targetIds: [targetSegments[index].id],
  status: "provisional",
}));
