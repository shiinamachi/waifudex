import { style } from "@vanilla-extract/css";
import { tokens } from "@fluentui/react-components";

export const previewContainer = style({
  position: "relative",
  backgroundColor: tokens.colorNeutralBackground4,
  borderRadius: tokens.borderRadiusSmall,
  border: `1px solid ${tokens.colorNeutralStroke2}`,
  overflow: "hidden",
  marginTop: tokens.spacingVerticalS,
});

export const characterArea = style({
  position: "absolute",
  bottom: 0,
  right: "10%",
  border: `2px dashed ${tokens.colorBrandStroke1}`,
  backgroundColor: tokens.colorBrandBackground2,
  borderRadius: tokens.borderRadiusSmall,
});

export const monitorLabel = style({
  position: "absolute",
  top: tokens.spacingVerticalXS,
  right: tokens.spacingHorizontalXS,
  maxWidth: "calc(100% - 16px)",
  padding: `2px ${tokens.spacingHorizontalXS}`,
  fontSize: "10px",
  color: tokens.colorNeutralForeground3,
  backgroundColor: tokens.colorNeutralBackground1,
  borderRadius: tokens.borderRadiusSmall,
  overflow: "hidden",
  textOverflow: "ellipsis",
  whiteSpace: "nowrap",
});

export const monitorResolutionLabel = style({
  position: "absolute",
  top: tokens.spacingVerticalXS,
  left: tokens.spacingHorizontalXS,
  fontSize: "10px",
  color: tokens.colorNeutralForeground4,
});
