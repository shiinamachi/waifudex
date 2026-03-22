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
  display: "flex",
  flexDirection: "column",
  height: "100%",
  minHeight: 0,
  overflow: "hidden",
});

export const contents = style({
  display: "flex",
  flex: 1,
  flexDirection: "column",
  height: "100%",
  minHeight: 0,
  overflow: "hidden",
  width: "100%",
});

export const layoutRoot = style({
  display: "flex",
  flexDirection: "column",
  height: "100%",
  minHeight: 0,
  overflow: "hidden",
});
