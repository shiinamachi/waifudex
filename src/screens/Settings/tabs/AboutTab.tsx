import { Button, Caption1 } from "@fluentui/react-components";
import { useState } from "react";

import SettingItem from "../../../components/settings/SettingItem";
import { useAppUpdate } from "../../../hooks/useAppUpdate";
import {
  tabsContainer,
  updateActions,
  updateMeta,
  updateStatus,
} from "./tabs.css";
import OpenSourceLicenseTab from "./OpenSourceLicenseTab";

type AboutSubPage = null | "open-source-license";

export default function AboutTab() {
  const [subPage, setSubPage] = useState<AboutSubPage>(null);
  const appUpdate = useAppUpdate();

  async function handleCheckForUpdates() {
    try {
      await appUpdate.checkForUpdates();
    } catch (error) {
      console.error("failed to check for updates", error);
    }
  }

  async function handleRestartToApply() {
    try {
      await appUpdate.restartToApply();
    } catch (error) {
      console.error("failed to restart to apply update", error);
    }
  }

  if (subPage === "open-source-license") {
    return <OpenSourceLicenseTab onBack={() => setSubPage(null)} />;
  }

  return (
    <div className={tabsContainer}>
      <SettingItem title="Version" type="text" value={appUpdate.currentVersion} />
      <SettingItem
        title="Updates"
        description="waifudex checks for updates automatically on startup. You can also inspect the current updater state here."
      >
        <Caption1 className={updateStatus}>{appUpdate.statusText}</Caption1>
        {appUpdate.availableVersion ? (
          <Caption1 className={updateMeta}>
            Latest version: {appUpdate.availableVersion}
          </Caption1>
        ) : null}
        {appUpdate.lastCheckedAt ? (
          <Caption1 className={updateMeta}>
            Last checked: {new Date(appUpdate.lastCheckedAt).toLocaleString()}
          </Caption1>
        ) : null}
        <div className={updateActions}>
          <Button
            appearance="secondary"
            disabled={!appUpdate.isLoaded || appUpdate.isChecking || appUpdate.isReadyToRestart}
            onClick={() => void handleCheckForUpdates()}
          >
            Check for updates
          </Button>
          {appUpdate.isReadyToRestart ? (
            <Button appearance="primary" onClick={() => void handleRestartToApply()}>
              Restart now
            </Button>
          ) : null}
        </div>
      </SettingItem>
      <SettingItem
        title="Open Source License"
        description="waifudex was developed using various open source technologies.\nWe acknowledge and respect the copyright and license terms of each project."
        type="link"
        onAction={() => setSubPage("open-source-license")}
      />
      <SettingItem
        title="Github"
        type="link"
        link="https://github.com/shiinamachi/waifudex"
      />
      <SettingItem
        title="Donate"
        description="waifudex is developed and maintained by a team of independent developers. Your support is greatly appreciated and helps keep the project running."
        type="link"
        link="https://github.com/sponsors/shiinamachi"
      />
    </div>
  );
}
