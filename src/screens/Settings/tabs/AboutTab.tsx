import SettingItem from "../../../components/settings/SettingItem";
import { tabsContainer } from "./tabs.css";
import packageJson from "../../../../package.json";

export default function AboutTab() {
  return (
    <div className={tabsContainer}>
      <SettingItem title="Version" type="text" value={packageJson.version} />
      <SettingItem
        title="Open Source License"
        description="waifudex was developed using various open source technologies.\nWe acknowledge and respect the copyright and license terms of each project."
      />
      <SettingItem
        title="Github"
        type="link"
        link="https://github.com/shiinamachi/waifudex"
      />
    </div>
  );
}
