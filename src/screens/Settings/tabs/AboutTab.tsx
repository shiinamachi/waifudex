import SettingItem from "../../../components/settings/SettingItem";
import { tabsContainer } from "./tabs.css";
import packageJson from "../../../../package.json";

export default function AboutTab() {
  return (
    <div className={tabsContainer}>
      <SettingItem
        title="Version"
        description="waifudex App version"
        type="text"
        value={packageJson.version}
      />
    </div>
  );
}
