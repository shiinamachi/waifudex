import { globalStyle, style } from "@vanilla-extract/css";

export const winUiSwitch = style({});

globalStyle(`${winUiSwitch}.fui-Switch`, {
  vars: {
    "--waifudex-switch-track-width": "40px",
    "--waifudex-switch-thumb-size": "18px",
    "--waifudex-switch-inner-gap": "2px",
    "--waifudex-switch-thumb-travel":
      "calc(var(--waifudex-switch-track-width) - var(--waifudex-switch-thumb-size) - var(--waifudex-switch-inner-gap))",
  },
  alignItems: "center",
});

globalStyle(`${winUiSwitch} .fui-Switch__indicator`, {
  transitionDuration: "83ms",
  transitionProperty: "background-color, border-color, color, opacity",
  transitionTimingFunction: "cubic-bezier(0, 0, 0, 1)",
});

globalStyle(`${winUiSwitch} .fui-Switch__indicator > svg`, {
  transform: "translateX(0)",
  transitionDuration: "83ms",
  transitionProperty: "transform",
  transitionTimingFunction: "cubic-bezier(0, 0, 0, 1)",
});

globalStyle(`${winUiSwitch} .fui-Switch__input:checked ~ .fui-Switch__indicator > svg`, {
  transform: "translateX(var(--waifudex-switch-thumb-travel))",
});

globalStyle("@media screen and (prefers-reduced-motion: reduce)", {
  [`${winUiSwitch} .fui-Switch__indicator`]: {
    transitionDuration: "0.01ms",
  },
  [`${winUiSwitch} .fui-Switch__indicator > svg`]: {
    transitionDuration: "0.01ms",
  },
});
