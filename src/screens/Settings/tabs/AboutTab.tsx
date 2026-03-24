import { useState } from "react";

import SettingItem from "../../../components/settings/SettingItem";
import { tabsContainer } from "./tabs.css";
import packageJson from "../../../../package.json";
import OpenSourceLicenseTab from "./OpenSourceLicenseTab";

type AboutSubPage = null | "open-source-license";

export default function AboutTab() {
  const [subPage, setSubPage] = useState<AboutSubPage>(null);

  if (subPage === "open-source-license") {
    return <OpenSourceLicenseTab onBack={() => setSubPage(null)} />;
  }

  return (
    <div className={tabsContainer}>
      <SettingItem title="Version" type="text" value={packageJson.version} />
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
