import { style } from "@vanilla-extract/css";
import { tokens } from "@fluentui/react-components";

export const layout = style({
  display: "grid",
  gap: "16px",
  gridTemplateColumns: "180px minmax(0, 1fr)",
  height: "100%",
});

export const tabList = style({
  alignSelf: "start",
  height: "100%",
  padding: 8,
});

export const panel = style({
  backgroundColor: tokens.colorNeutralBackground2,
  borderRadius: tokens.borderRadiusLarge,
  boxShadow: tokens.shadow4,
  height: "100%",
  padding: "16px",
});
