import "./style.css";
import catRig from "../../assets/companion/puppets/v1/orange-cat/rig.json";
import dogRig from "../../assets/companion/puppets/v1/golden-dog/rig.json";
import motions from "../../assets/companion/puppets/v1/motions.json";
import catMaster from "../../assets/companion/sprites/v1/orange-cat/frames/00.png";
import dogMaster from "../../assets/companion/sprites/v1/golden-dog/frames/00.png";
import catAtlas from "../../assets/companion/sprites/v1/orange-cat/atlas.webp";
import dogAtlas from "../../assets/companion/sprites/v1/golden-dog/atlas.webp";
import spriteMotions from "../../assets/companion/sprites/v1/motions.json";
import { sampleSprite, type SpriteLibrary } from "../../domain/companion/puppet/sprite";
import { samplePuppet } from "../../domain/companion/puppet/sample";
import type { PuppetMotionLibrary, PuppetRig } from "../../domain/companion/puppet/types";

// Development-only entry. The application barrel never imports this asset pack.
const slices = import.meta.glob<string>("../../assets/companion/puppets/v1/*/parts/*.svg", { eager: true, query: "?raw", import: "default" });
const library = motions as PuppetMotionLibrary;
const spriteLibrary = spriteMotions as SpriteLibrary;
const atlases: Record<string, string> = { "orange-cat": catAtlas, "golden-dog": dogAtlas };
const gifUrls = import.meta.glob<string>("../../assets/companion/sprites/v1/*/previews/*.gif", { eager: true, query: "?url", import: "default" });
const frameUrls = import.meta.glob<string>("../../assets/companion/sprites/v1/*/frames/*.png", { eager: true, query: "?url", import: "default" });
const rigs: Record<string, PuppetRig> = { "orange-cat": catRig, "golden-dog": dogRig };
const namespace = "http://www.w3.org/2000/svg";
const root = document.querySelector<HTMLDivElement>("#companion-lab")!;
root.innerHTML = `
  <header class="page-header"><a class="wordmark" href="/">决明<span>JUEMING</span></a><span class="header-rule"></span><span>桌宠素材工坊</span><span class="version">ANIME DEV · 01</span></header>
  <main>
    <div class="intro"><div><p class="eyebrow">COMPANION ATELIER</p><h1>小伙伴的动作工坊</h1><p class="intro-copy">选择一个小伙伴，看看它走路、跳跃和睡觉的样子。</p></div><span class="pack-note">参考插画 · 混合动画<span>帧动画 + SVG 分件</span></span></div>
    <div class="workbench">
      <aside class="cast panel"><p class="section-label">01 / 角色</p><button class="actor-card selected" data-actor="orange-cat" aria-pressed="true"><img src="${catMaster}" alt="胖橘猫"/><span><strong>胖橘猫</strong><small>ORANGE TABBY</small></span><span class="count">16 帧</span></button><button class="actor-card" data-actor="golden-dog" aria-pressed="false"><img src="${dogMaster}" alt="金毛"/><span><strong>金毛</strong><small>GOLDEN RETRIEVER</small></span><span class="count">16 帧</span></button></aside>
      <section class="viewer panel" aria-label="角色预览">
        <div class="viewer-top"><div class="view-tabs" role="group" aria-label="预览方式"><button id="assembled" class="selected" aria-pressed="true">插画动画</button><button id="rig-view" aria-pressed="false">SVG 绑定</button><button id="separated" aria-pressed="false">全部切片</button></div><span id="actor-summary"></span></div>
        <div class="stage-wrap"><svg id="stage" viewBox="0 0 900 430" role="img" aria-label="桌宠动作预览"><defs><pattern id="grid" width="30" height="30" patternUnits="userSpaceOnUse"><circle cx="1" cy="1" r=".75" fill="var(--grid)"/></pattern></defs><rect width="900" height="430" fill="url(#grid)"/><text x="27" y="34" class="stage-caption">MOVEMENT STUDY</text><g id="scene-world"><path d="M 45 334 Q 420 322 855 334" fill="none" stroke="var(--ground)" stroke-width="1.5"/><g id="bed-back"><path d="M 323 252 L 347 320 L 352 327 M 577 252 L 554 320 L 549 327" fill="none" stroke="#BCAA83" stroke-width="9" stroke-linecap="round"/><ellipse cx="450" cy="251" rx="135" ry="26" fill="#D0BD98"/><ellipse cx="450" cy="245" rx="120" ry="21" fill="#F2E5CA"/></g><g id="garden"><path d="M 674 330 Q 663 287 681 258 M 681 329 Q 704 294 700 273 M 703 330 Q 722 312 731 295" fill="none" stroke="#79965D" stroke-width="4" stroke-linecap="round"/><path d="M 674 293 Q 645 297 647 275 Q 671 272 674 293 M 678 275 Q 698 273 696 253 Q 676 255 678 275 M 704 301 Q 728 302 726 280 Q 706 279 704 301 M 716 318 Q 742 324 747 309 Q 729 300 716 318" fill="#9DAF73"/><g fill="#E3BD59"><circle cx="681" cy="251" r="7"/><circle cx="670" cy="259" r="7"/><circle cx="685" cy="264" r="7"/><circle cx="698" cy="267" r="6"/></g><circle cx="680" cy="258" r="4" fill="#B88B3F"/></g><ellipse id="shadow" cx="230" cy="334" rx="92" ry="7" fill="#9E8F66" opacity=".12"/><g id="puppet"></g><g id="sprite"><svg id="sprite-previous" x="-160" y="-293.333333" width="320" height="320"><image width="1536" height="1536"/></svg><svg id="sprite-current" x="-160" y="-293.333333" width="320" height="320"><image width="1536" height="1536"/></svg></g><g id="joints" aria-hidden="true"></g></g><text x="27" y="407" id="stage-label" class="stage-caption"></text></svg><div id="slice-grid" class="slice-grid" hidden></div></div>
        <div class="viewer-bottom"><label><input type="checkbox" id="show-joints"/> 显示关节</label><label>朝向 <select id="direction"><option value="1">向右 →</option><option value="-1">← 向左</option></select></label><button id="clear-part" type="button">取消部件高亮</button></div>
      </section>
      <aside class="sprite-inspector inspector panel"><p class="section-label">02 / 动作素材</p><div class="sprite-poster"><img id="frame-preview" alt="当前动作帧"/></div><h2 id="sprite-action-name">坐着</h2><p id="sprite-frame-id" class="mono"></p><p class="sprite-note">起身、迈步、蜷睡。把每个小动作画下来，串成一段完整的动画。</p><a id="download-frame" class="download" download>下载当前 PNG 帧 <span>↓</span></a><a id="download-gif" class="download" download>下载动作 GIF <span>↓</span></a><p class="inspector-hint">GIF 播放一次；页面中的动画可以暂停或拖动查看。</p></aside>
      <aside class="rig-inspector inspector panel" hidden><p class="section-label">02 / 部件检查</p><div id="part-preview" class="part-preview"></div><h2 id="part-name"></h2><p id="part-id" class="mono"></p><dl><div><dt>绑定关节</dt><dd id="part-bone"></dd></div><div><dt>父级关节</dt><dd id="part-parent"></dd></div><div><dt>旋转原点</dt><dd>0, 0 · 局部坐标</dd></div><div><dt>资源格式</dt><dd>透明 SVG</dd></div></dl><a id="download-part" class="download" download>下载当前切片 <span>↓</span></a><p class="inspector-hint">点击角色或「全部切片」中的部件，查看对应的绑定信息。</p></aside>
    <section class="motion-panel panel" aria-label="动作控制"><div class="motion-heading"><p class="section-label">03 / 动作</p><p id="action-description"></p></div><div class="actions" role="group" aria-label="选择动作"></div><div class="transport"><button id="play" class="play">播放一次</button><button id="reset" class="reset">复位</button><label class="timeline-label" for="timeline">动作进度</label><input id="timeline" type="range" min="0" max="1000" value="0" step="1"/><output id="timecode">0.00 / 1.00 s</output><label class="speed-label">速度 <select id="speed"><option value="0.5">0.5×</option><option value="1" selected>1×</option><option value="1.5">1.5×</option></select></label></div><p id="playback-status" class="playback-status" role="status">按需播放 · 每次动作结束后停下</p></section>
    </div>
    <footer><span>插画关键帧 / SVG 分件 / 有限动作 / 场景路径</span><span>素材与动作分离 / 开发预览</span></footer>
  </main>`;

function element<T extends Element>(id: string): T { return document.getElementById(id) as unknown as T; }
function svgElement(tag: string, attrs: Record<string, string> = {}): SVGElement {
  const node = document.createElementNS(namespace, tag);
  for (const [key, value] of Object.entries(attrs)) node.setAttribute(key, value);
  return node;
}
function sliceSvg(rig: PuppetRig, id: string): SVGSVGElement {
  const raw = slices[`../../assets/companion/puppets/v1/${rig.id}/parts/${id}.svg`];
  if (!raw) throw new Error(`Missing SVG slice: ${rig.id}/${id}`);
  const parsed = new DOMParser().parseFromString(raw, "image/svg+xml");
  return document.importNode(parsed.documentElement, true) as unknown as SVGSVGElement;
}

let rig = rigs["orange-cat"];
let actionId = "stand";
let renderMode: "sprite" | "rig" = "sprite";
function currentActions() { return renderMode === "sprite" ? spriteLibrary.actions : library.actions; }
let elapsed = 0;
let playing = false;
let frameHandle = 0;
let previousTime = 0;
let selectedPart: string | null = null;
let downloadUrl: string | null = null;
let partNodes = new Map<string, SVGElement>();
const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
const puppet = element<SVGGElement>("puppet");
const jointLayer = element<SVGGElement>("joints");
const timeline = element<HTMLInputElement>("timeline");
const playButton = element<HTMLButtonElement>("play");
const actionButtons = root.querySelector<HTMLDivElement>(".actions")!;

function mountActions() {
  actionButtons.replaceChildren();
  for (const [id, action] of Object.entries(currentActions())) {
    const button = document.createElement("button");
    button.type = "button";
    button.dataset.action = id;
    button.innerHTML = `<strong>${action.label}</strong><small>${(action.durationMs / 1000).toFixed(1)} s</small>`;
    button.addEventListener("click", () => {
      pause(); actionId = id; elapsed = 0; refresh();
      element("playback-status").textContent = "动作已就绪，可播放或拖动进度检查姿态";
    });
    actionButtons.append(button);
  }
}

function inspect(id: string | null) {
  selectedPart = id;
  const part = rig.parts.find(item => item.id === id) ?? rig.parts.find(item => item.id === "body")!;
  element("part-preview").replaceChildren(sliceSvg(rig, part.id));
  element("part-name").textContent = part.label;
  element("part-id").textContent = part.id;
  element("part-bone").textContent = part.bone;
  element("part-parent").textContent = rig.bones.find(item => item.id === part.bone)?.parent ?? "—";
  if (downloadUrl) URL.revokeObjectURL(downloadUrl);
  downloadUrl = URL.createObjectURL(new Blob([slices[`../../assets/companion/puppets/v1/${rig.id}/${part.file}`]], { type: "image/svg+xml" }));
  const download = element<HTMLAnchorElement>("download-part");
  download.href = downloadUrl;
  download.download = `${rig.id}-${part.id}.svg`;
  refresh();
}

function mountRig() {
  puppet.replaceChildren(); partNodes = new Map();
  const grid = element("slice-grid"); grid.replaceChildren();
  for (const part of rig.parts) {
    const source = sliceSvg(rig, part.id);
    const group = svgElement("g", { "data-part": part.id, "aria-label": part.label, fill: "none" });
    // Preserve geometry in joint-local coordinates; the SVG viewport is only for slicing.
    group.append(...Array.from(source.children));
    group.addEventListener("click", () => inspect(part.id));
    puppet.append(group); partNodes.set(part.id, group);
    const button = document.createElement("button");
    button.type = "button"; button.dataset.slice = part.id;
    button.append(sliceSvg(rig, part.id));
    const label = document.createElement("span"); label.textContent = part.label; button.append(label);
    button.addEventListener("click", () => inspect(part.id)); grid.append(button);
  }
  element("actor-summary").textContent = `${rig.parts.length} 切片 / ${rig.bones.length} 关节`;
  inspect(null);
}

function refresh() {
  const action = currentActions()[actionId];
  const illustrated = renderMode === "sprite";
  const closeUp = ["stand", "sleep", "rise", "sit-down", "wake-up"].includes(actionId);
  let x: number, scale: number;
  puppet.setAttribute("display", illustrated ? "none" : "inline");
  element("sprite").setAttribute("display", illustrated ? "inline" : "none");
  jointLayer.replaceChildren();
  if (illustrated) {
    const frame = sampleSprite(spriteLibrary, actionId, elapsed);
    scale = closeUp ? 1.25 : 1;
    x = (closeUp ? 450 : 215) + frame.displacement.x;
    element("sprite").setAttribute("transform", `translate(${x} ${334 + frame.displacement.y}) scale(${scale})`);
    for (const [id, index, opacity] of [["sprite-previous", frame.previous, 1 - frame.mix], ["sprite-current", frame.current, frame.mix]] as const) {
      const viewport = element<SVGSVGElement>(id);
      viewport.setAttribute("viewBox", `${index % 4 * 384} ${Math.floor(index / 4) * 384} 384 384`);
      viewport.setAttribute("opacity", String(opacity));
      viewport.querySelector("image")!.setAttribute("href", atlases[rig.id]);
    }
    const frameUrl = frameUrls[`../../assets/companion/sprites/v1/${rig.id}/frames/${String(frame.current).padStart(2, "0")}.png`];
    element<HTMLImageElement>("frame-preview").src = frameUrl;
    element("sprite-action-name").textContent = action.label;
    element("sprite-frame-id").textContent = `FRAME ${String(frame.current + 1).padStart(2, "0")} / 16`;
    element<HTMLAnchorElement>("download-frame").href = frameUrl;
    element<HTMLAnchorElement>("download-frame").download = `${rig.id}-${frame.current}.png`;
    const gif = gifUrls[`../../assets/companion/sprites/v1/${rig.id}/previews/${actionId}.gif`];
    const download = element<HTMLAnchorElement>("download-gif");
    download.hidden = !gif;
    if (gif) { download.href = gif; download.download = `${rig.id}-${actionId}.gif`; }
  } else {
    const frame = samplePuppet(rig, library, actionId, elapsed);
    scale = closeUp ? 1.6 : 1;
    x = (closeUp ? 430 : 215) + frame.displacement.x;
    const y = 334 - (rig.groundY - rig.origin[1]) * scale + frame.displacement.y;
    const actorTransform = `translate(${x} ${y}) scale(${scale})`;
    puppet.setAttribute("transform", actorTransform); jointLayer.setAttribute("transform", actorTransform);
    for (const part of rig.parts) {
      const node = partNodes.get(part.id)!;
      node.setAttribute("transform", `matrix(${frame.bones.get(part.bone)!.join(" ")})`);
      node.setAttribute("display", !part.expressions || part.expressions.includes(frame.expression) ? "inline" : "none");
      node.setAttribute("opacity", selectedPart && selectedPart !== part.id ? ".25" : "1");
    }
    if (element<HTMLInputElement>("show-joints").checked) {
      for (const bone of rig.bones) {
        const matrix = frame.bones.get(bone.id)!;
        const parent = bone.parent ? frame.bones.get(bone.parent)! : matrix;
        jointLayer.append(svgElement("line", { x1: `${parent[4]}`, y1: `${parent[5]}`, x2: `${matrix[4]}`, y2: `${matrix[5]}`, stroke: "#35785E", "stroke-width": "1", opacity: ".65" }));
        jointLayer.append(svgElement("circle", { cx: `${matrix[4]}`, cy: `${matrix[5]}`, r: "3", fill: "#F9F9E9", stroke: "#35785E", "stroke-width": "1.5" }));
      }
    }
  }
  element<HTMLInputElement>("show-joints").disabled = illustrated;
  element<HTMLButtonElement>("clear-part").hidden = illustrated;
  root.querySelector<HTMLElement>(".rig-inspector")!.hidden = illustrated;
  root.querySelector<HTMLElement>(".sprite-inspector")!.hidden = !illustrated;
  element("actor-summary").textContent = illustrated ? "16 帧 / 插画动作序列" : `${rig.parts.length} 切片 / ${rig.bones.length} 关节`;
  element("shadow").setAttribute("cx", `${x}`);
  element("shadow").setAttribute("rx", `${85 * scale}`);
  element("shadow").setAttribute("cy", action.scene === "bed" && elapsed >= action.durationMs * .8 ? "254" : "334");
  element("bed-back").setAttribute("display", action.scene === "bed" ? "inline" : "none");
  element("garden").setAttribute("display", action.scene === "garden" ? "inline" : "none");
  element("scene-world").setAttribute("transform", element<HTMLSelectElement>("direction").value === "-1" ? "translate(900 0) scale(-1 1)" : "");
  element("stage-label").textContent = `${rig.label} / ${action.label}`;
  element("action-description").textContent = action.description;
  timeline.max = String(action.durationMs); timeline.value = String(elapsed);
  element("timecode").textContent = `${(elapsed / 1000).toFixed(2)} / ${(action.durationMs / 1000).toFixed(2)} s`;
  actionButtons.querySelectorAll<HTMLButtonElement>("button").forEach(button => {
    const active = button.dataset.action === actionId;
    button.classList.toggle("selected", active); button.setAttribute("aria-pressed", String(active));
  });
  root.querySelectorAll<HTMLButtonElement>("[data-slice]").forEach(button => {
    const active = button.dataset.slice === selectedPart;
    button.classList.toggle("selected", active); button.setAttribute("aria-pressed", String(active));
  });
}

function pause() {
  playing = false; cancelAnimationFrame(frameHandle); frameHandle = 0;
  playButton.textContent = "播放一次";
}
function tick(now: number) {
  if (!playing) return;
  const action = currentActions()[actionId];
  elapsed = Math.min(action.durationMs, elapsed + (now - previousTime) * Number(element<HTMLSelectElement>("speed").value));
  previousTime = now; refresh();
  if (elapsed >= action.durationMs) {
    pause(); element("playback-status").textContent = "动作已结束 · 保持最后姿态";
  } else frameHandle = requestAnimationFrame(tick);
}
playButton.addEventListener("click", () => {
  if (playing) { pause(); element("playback-status").textContent = "已暂停"; return; }
  if (reducedMotion.matches || document.hidden) return;
  if (elapsed >= currentActions()[actionId].durationMs) elapsed = 0;
  playing = true; previousTime = performance.now(); playButton.textContent = "暂停";
  element("playback-status").textContent = "正在播放 · 结束后自动停止";
  frameHandle = requestAnimationFrame(tick);
});
element("reset").addEventListener("click", () => { pause(); elapsed = 0; refresh(); element("playback-status").textContent = "已回到动作起点"; });
timeline.addEventListener("input", () => { pause(); elapsed = Number(timeline.value); refresh(); });
element("show-joints").addEventListener("change", refresh);
element("direction").addEventListener("change", refresh);
element("clear-part").addEventListener("click", () => inspect(null));
for (const button of root.querySelectorAll<HTMLButtonElement>("[data-actor]")) {
  button.addEventListener("click", () => {
    pause(); rig = rigs[button.dataset.actor!]; elapsed = 0;
    root.querySelectorAll<HTMLButtonElement>("[data-actor]").forEach(item => { const active = item === button; item.classList.toggle("selected", active); item.setAttribute("aria-pressed", String(active)); });
    mountRig();
  });
}
for (const id of ["assembled", "rig-view", "separated"]) {
  element(id).addEventListener("click", () => {
    pause(); const separated = id === "separated";
    renderMode = id === "assembled" ? "sprite" : "rig";
    if (!currentActions()[actionId]) actionId = "stand";
    elapsed = 0; mountActions(); refresh();
    element<HTMLElement>("stage").style.display = separated ? "none" : "block";
    element<HTMLElement>("slice-grid").hidden = !separated;
    for (const tab of ["assembled", "rig-view", "separated"]) { element(tab).classList.toggle("selected", tab === id); element(tab).setAttribute("aria-pressed", String(tab === id)); }
  });
}
function motionPolicy() {
  if (reducedMotion.matches) pause();
  playButton.disabled = reducedMotion.matches;
  element("playback-status").textContent = reducedMotion.matches ? "系统已减少动态 · 拖动进度可检查静态姿态" : "按需播放 · 每次动作结束后停下";
}
function pauseForBackground() { if (playing) { pause(); element("playback-status").textContent = "窗口已离开前台，动作暂停"; } }
reducedMotion.addEventListener("change", motionPolicy);
document.addEventListener("visibilitychange", () => { if (document.hidden) pauseForBackground(); });
window.addEventListener("blur", pauseForBackground);
window.addEventListener("pagehide", () => { pause(); if (downloadUrl) URL.revokeObjectURL(downloadUrl); });
mountActions(); mountRig(); motionPolicy();
