import { tokens } from "@fluentui/react-components";
import { style } from "@vanilla-extract/css";

export const tabsContainer = style({
  display: "flex",
  flexDirection: "column",
  gap: tokens.spacingVerticalS,
});

export const updateActions = style({
  display: "flex",
  flexWrap: "wrap",
  gap: tokens.spacingHorizontalS,
  marginTop: tokens.spacingVerticalS,
});

export const updateMeta = style({
  color: tokens.colorNeutralForeground3,
  marginTop: tokens.spacingVerticalXS,
});

export const updateStatus = style({
  color: tokens.colorNeutralForeground2,
});

export const modelListHeader = style({
  display: "flex",
  justifyContent: "flex-end",
  marginBottom: tokens.spacingVerticalXS,
});

export const modelCard = style({
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
  gap: tokens.spacingHorizontalM,
});

export const modelCardInfo = style({
  display: "flex",
  flexDirection: "column",
  flex: 1,
  minWidth: 0,
});

export const modelCardActions = style({
  display: "flex",
  alignItems: "center",
  gap: tokens.spacingHorizontalXS,
  flexShrink: 0,
});
