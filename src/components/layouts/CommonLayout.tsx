import type { PropsWithChildren } from "react";
import { useEffect, useState } from "react";
import {
  getCurrentWindow,
  type Theme as WindowTheme,
} from "@tauri-apps/api/window";
import {
  FluentProvider,
  webDarkTheme,
  webLightTheme,
} from "@fluentui/react-components";
import "the-new-css-reset";

import { contents, layoutRoot, providerRoot } from "./common-layout.css";

function systemThemeFallback(): WindowTheme {
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

export default function CommonLayout({ children }: PropsWithChildren) {
  const [windowTheme, setWindowTheme] =
    useState<WindowTheme>(systemThemeFallback);

  useEffect(() => {
    const currentWindow = getCurrentWindow();
    let unlistenThemeChange: (() => void) | undefined;
    let disposed = false;

    void currentWindow.theme().then((theme) => {
      if (!disposed) {
        setWindowTheme(theme ?? systemThemeFallback());
      }
    });

    void currentWindow
      .onThemeChanged(({ payload }) => {
        if (!disposed) {
          setWindowTheme(payload ?? systemThemeFallback());
        }
      })
      .then((unlisten) => {
        if (disposed) {
          unlisten();
          return;
        }

        unlistenThemeChange = unlisten;
      });

    return () => {
      disposed = true;
      unlistenThemeChange?.();
    };
  }, []);

  return (
    <div className={layoutRoot}>
      <FluentProvider
        style={{ height: "100%" }}
        theme={windowTheme === "dark" ? webDarkTheme : webLightTheme}
      >
        <div className={providerRoot}>
          <div className={contents}>{children}</div>
        </div>
      </FluentProvider>
    </div>
  );
}
