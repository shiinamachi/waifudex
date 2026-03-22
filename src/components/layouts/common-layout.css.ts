import { globalStyle, style } from "@vanilla-extract/css";
import { tokens } from "@fluentui/react-components";

globalStyle(":root", {
  colorScheme: "light dark",
});

globalStyle("html", {
  height: "100%",
  background: "Canvas",
  color: "CanvasText",
});

globalStyle("body", {
  height: "100%",
  background: "Canvas",
  color: "CanvasText",
});

globalStyle("#app", {
  height: "100%",
});

export const providerRoot = style({
  backgroundColor: tokens.colorNeutralBackground1,
  color: tokens.colorNeutralForeground1,
});

export const contents = style({
  height: "100%",
});

export const layoutRoot = style({
  height: "100%",
});
