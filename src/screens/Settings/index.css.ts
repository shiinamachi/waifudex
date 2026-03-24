import { globalStyle, style } from "@vanilla-extract/css";
import { tokens } from "@fluentui/react-components";

export const layout = style({
  display: "grid",
  gridTemplateColumns: "180px minmax(0, 1fr)",
  height: "100%",
  minHeight: 0,
  minWidth: 0,
  width: "100%",
});

export const tabList = style({
  alignSelf: "start",
  boxSizing: "border-box",
  display: "flex",
  flexDirection: "column",
  height: "100%",
  minHeight: 0,
  padding: tokens.spacingHorizontalM,
});

export const tabListFooter = style({
  display: "flex",
  flexDirection: "column",
  marginTop: "auto",
  width: "100%",
});

export const tabListFooterTab = style({
  justifyContent: "flex-start",
  width: "100%",
});

export const panel = style({
  backgroundColor: tokens.colorNeutralBackground2,
  borderRadius: tokens.borderRadiusLarge,
  boxShadow: tokens.shadow4,
  display: "flex",
  height: "100%",
  minHeight: 0,
  minWidth: 0,
  overflow: "hidden",
});

export const panelScroll = style({
  flex: 1,
  minHeight: 0,
  minWidth: 0,
  overflowX: "hidden",
  overflowY: "auto",
  padding: tokens.spacingHorizontalM,
  width: "100%",
  boxSizing: "border-box",
  scrollbarColor: `${tokens.colorNeutralForeground4} transparent`,
  scrollbarGutter: "stable",
  scrollbarWidth: "thin",
});

globalStyle(`${panelScroll}::-webkit-scrollbar`, {
  width: "8px",
});

globalStyle(`${panelScroll}::-webkit-scrollbar-track`, {
  backgroundColor: "transparent",
});

globalStyle(`${panelScroll}::-webkit-scrollbar-thumb`, {
  backgroundClip: "padding-box",
  backgroundColor: tokens.colorNeutralForeground4,
  border: "2px solid transparent",
  borderRadius: tokens.borderRadiusCircular,
  minHeight: "48px",
});

globalStyle(`${panelScroll}:hover`, {
  scrollbarColor: `${tokens.colorNeutralForeground3} transparent`,
});

globalStyle(`${panelScroll}:hover::-webkit-scrollbar`, {
  width: "12px",
});

globalStyle(`${panelScroll}:hover::-webkit-scrollbar-thumb`, {
  backgroundColor: tokens.colorNeutralForeground3,
  border: `2px solid ${tokens.colorNeutralBackground2}`,
});

globalStyle(`${panelScroll}::-webkit-scrollbar-thumb:hover`, {
  backgroundColor: tokens.colorNeutralForeground2,
});

globalStyle(`${panelScroll}::-webkit-scrollbar-thumb:active`, {
  backgroundColor: tokens.colorNeutralForeground1,
  border: `2px solid ${tokens.colorNeutralBackground2}`,
});

globalStyle(`${panelScroll}::-webkit-scrollbar-corner`, {
  backgroundColor: "transparent",
});
