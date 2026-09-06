import type { CompanionActor, CompanionRenderer, CompanionRendererState } from "./types";

const actorElement = (root: HTMLElement, actor: CompanionActor) => root.querySelector<HTMLElement>(`[data-companion-actor="${actor}"]`);

const finiteAnimation = (element: HTMLElement | null, keyframes: Keyframe[], duration: number) => {
  if (!element || typeof element.animate !== "function") return null;
  return element.animate(keyframes, { duration, easing: "ease-in-out", fill: "none", iterations: 1 });
};

/**
 * A deliberately small optional renderer. It only decorates static actor DOM
 * supplied by Vue and owns every Animation it creates.
 */
export class WebAnimationsCompanionRenderer implements CompanionRenderer {
  private root: HTMLElement | null = null;
  private animations: Animation[] = [];

  mount(root: HTMLElement) {
    this.root = root;
  }

  update(state: CompanionRendererState) {
    this.cancel();
    if (!this.root) return;
    const cat = actorElement(this.root, "cat");
    const dog = actorElement(this.root, "dog");
    const plant = actorElement(this.root, "plant");
    const butterfly = actorElement(this.root, "butterfly");
    const add = (animation: Animation | null) => { if (animation) this.animations.push(animation); };

    switch (state.activity) {
      case "idle":
        add(finiteAnimation(cat, [{ transform: "translateY(0px)" }, { transform: "translateY(-3px)" }, { transform: "translateY(0px)" }], 760));
        add(finiteAnimation(dog, [{ transform: "translateY(0px)" }, { transform: "translateY(-2px)" }, { transform: "translateY(0px)" }], 680));
        add(finiteAnimation(plant, [
          { transform: "rotate(0deg)" },
          { transform: "rotate(-1.8deg)" },
          { transform: "rotate(1.8deg)" },
          { transform: "rotate(-1.2deg)" },
          { transform: "rotate(0deg)" },
        ], 3600));
        break;
      case "running":
        add(finiteAnimation(cat, [{ transform: "translateX(0px)" }, { transform: "translateX(5px)" }, { transform: "translateX(0px)" }], 520));
        add(finiteAnimation(dog, [{ transform: "rotate(0deg)" }, { transform: "rotate(2deg)" }, { transform: "rotate(0deg)" }], 560));
        break;
      case "awaiting_approval":
        add(finiteAnimation(plant, [{ transform: "scale(1)" }, { transform: "scale(1.025)" }, { transform: "scale(1)" }], 840));
        break;
      case "succeeded":
        add(finiteAnimation(butterfly, [{ transform: "translate(0px, 0px)" }, { transform: "translate(6px, -5px)" }, { transform: "translate(10px, -2px)" }], 920));
        break;
      case "failed":
        add(finiteAnimation(cat, [{ transform: "translateX(0px)" }, { transform: "translateX(-3px)" }, { transform: "translateX(3px)" }, { transform: "translateX(0px)" }], 360));
        break;
      case "cancelled":
        add(finiteAnimation(dog, [{ transform: "translateY(0px)" }, { transform: "translateY(2px)" }, { transform: "translateY(0px)" }], 440));
        break;
    }
  }

  pet(actor: CompanionActor) {
    this.cancel();
    if (!this.root) return;
    const animation = finiteAnimation(actorElement(this.root, actor), [
      { transform: "scale(1)" },
      { transform: "scale(1.04) translateY(-2px)" },
      { transform: "scale(1)" },
    ], 300);
    if (animation) this.animations.push(animation);
  }

  cancel() {
    for (const animation of this.animations) animation.cancel();
    this.animations = [];
  }

  dispose() {
    this.cancel();
    this.root = null;
  }
}

export const createWebAnimationsCompanionRenderer = () => new WebAnimationsCompanionRenderer();
