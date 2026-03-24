import { globalStyle, style } from "@vanilla-extract/css";
import { tokens } from "@fluentui/react-components";

export const container = style({
  display: "flex",
  flexDirection: "column",
  gap: tokens.spacingVerticalM,
  paddingBottom: tokens.spacingVerticalS,
});

export const stickyHeader = style({
  backgroundColor: tokens.colorNeutralBackground2,
  borderBottom: `1px solid ${tokens.colorNeutralStroke2}`,
  left: 0,
  marginBottom: tokens.spacingVerticalXS,
  marginLeft: `calc(${tokens.spacingHorizontalM} * -1)`,
  marginRight: `calc(${tokens.spacingHorizontalM} * -1)`,
  marginTop: `calc(${tokens.spacingHorizontalM} * -1)`,
  padding: `${tokens.spacingHorizontalM} ${tokens.spacingHorizontalM} ${tokens.spacingVerticalS}`,
  position: "sticky",
  right: 0,
  top: `calc(${tokens.spacingHorizontalM} * -1)`,
  zIndex: 1,
  "@supports": {
    "((backdrop-filter: blur(1px)) or (-webkit-backdrop-filter: blur(1px)))":
      {
        backgroundColor: `color-mix(in srgb, ${tokens.colorNeutralBackground2} 86%, transparent)`,
        backdropFilter: "blur(12px)",
        WebkitBackdropFilter: "blur(12px)",
      },
  },
});

export const header = style({
  alignItems: "flex-start",
  display: "flex",
  gap: tokens.spacingHorizontalS,
});

export const headerContent = style({
  display: "flex",
  flexDirection: "column",
  gap: tokens.spacingVerticalXS,
  minWidth: 0,
});

export const pageTitle = style({
  color: tokens.colorNeutralForeground2,
});

export const notice = style({
  color: tokens.colorNeutralForeground3,
  maxWidth: "60ch",
});

export const section = style({
  display: "flex",
  flexDirection: "column",
  gap: tokens.spacingVerticalS,
});

export const sectionHeader = style({
  alignItems: "baseline",
  display: "flex",
  gap: tokens.spacingHorizontalS,
  justifyContent: "space-between",
});

export const sectionTitle = style({
  color: tokens.colorNeutralForeground2,
});

export const sectionMeta = style({
  color: tokens.colorNeutralForeground3,
});

export const summaryAccordion = style({
  display: "flex",
  flexDirection: "column",
  gap: tokens.spacingVerticalXS,
});

export const packageList = style({
  display: "flex",
  flexDirection: "column",
  gap: tokens.spacingVerticalXS,
});

export const packageCard = style({
  backgroundColor: tokens.colorNeutralBackground1,
  border: `1px solid ${tokens.colorNeutralStroke2}`,
  boxShadow: "none",
});

export const packageHeader = style({
  display: "flex",
  justifyContent: "space-between",
  alignItems: "baseline",
  width: "100%",
});

export const packageMeta = style({
  color: tokens.colorNeutralForeground3,
  display: "flex",
  alignItems: "center",
  flexWrap: "wrap",
  gap: tokens.spacingHorizontalXS,
});

export const repositoryLink = style({
  color: tokens.colorBrandForegroundLink,
  cursor: "pointer",
  selectors: {
    "&:hover": {
      textDecoration: "underline",
    },
  },
});

export const accordionPanel = style({
  whiteSpace: "pre-wrap",
  fontFamily: tokens.fontFamilyMonospace,
  fontSize: tokens.fontSizeBase200,
  lineHeight: tokens.lineHeightBase200,
  maxHeight: "300px",
  overflowY: "auto",
  padding: tokens.spacingVerticalS,
});

export const accordionPanelText = style({
  display: "block",
  margin: 0,
  whiteSpace: "pre-wrap",
  overflowWrap: "anywhere",
  fontFamily: "inherit",
  fontSize: "inherit",
  lineHeight: "inherit",
});

export const badgeStyle = style({
  marginLeft: "auto",
});

export const accordionHeaderContent = style({
  display: "flex",
  justifyContent: "space-between",
  alignItems: "center",
  width: "100%",
});

globalStyle(`${accordionPanel}::-webkit-scrollbar`, {
  width: "6px",
});

globalStyle(`${accordionPanel}::-webkit-scrollbar-thumb`, {
  backgroundColor: tokens.colorNeutralForeground4,
  borderRadius: tokens.borderRadiusCircular,
});

globalStyle(`${summaryAccordion} .fui-AccordionItem`, {
  backgroundColor: tokens.colorNeutralBackground1,
  border: `1px solid ${tokens.colorNeutralStroke2}`,
  borderRadius: tokens.borderRadiusMedium,
  overflow: "hidden",
});
