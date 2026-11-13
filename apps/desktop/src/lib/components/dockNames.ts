// Names of panels and dock zones for labels and menus.
import { t } from "../i18n";
import type { PanelId, Zone } from "../dock";

export function panelName(p: PanelId): string {
  if (p === "side") return t("dock.side");
  if (p === "viewer") return t("dock.viewer");
  if (p === "plan") return t("plan.title");
  return t("term.title");
}

export function zoneName(z: Zone): string {
  if (z === "left") return t("dock.left");
  if (z === "center") return t("dock.center");
  if (z === "right") return t("dock.right");
  return t("dock.bottom");
}
