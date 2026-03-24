import { useState } from "react";
import {
  Divider,
  Tab,
  TabList,
  type SelectTabData,
  type SelectTabEvent,
  type TabValue,
} from "@fluentui/react-components";

import CommonLayout from "../../components/layouts/CommonLayout";
import {
  layout,
  panel,
  panelScroll,
  tabList,
  tabListFooter,
  tabListFooterTab,
} from "./index.css";
import SettingsDisplayTab from "./tabs/SettingsDisplayTab";
import AboutTab from "./tabs/AboutTab";

const DEFAULT_TAB: TabValue = "display";

export default function Settings() {
  const [selectedTab, setSelectedTab] = useState<TabValue>(DEFAULT_TAB);

  function handleTabSelect(_event: SelectTabEvent, data: SelectTabData): void {
    setSelectedTab(data.value);
  }

  return (
    <CommonLayout>
      <div className={layout}>
        <TabList
          appearance="transparent"
          className={tabList}
          onTabSelect={handleTabSelect}
          selectedValue={selectedTab}
          vertical
        >
          <Tab value="display">Display</Tab>
          <div className={tabListFooter}>
            <Divider />
            <Tab className={tabListFooterTab} value="about">
              About
            </Tab>
          </div>
        </TabList>

        <div className={panel}>
          <div className={panelScroll}>
            {selectedTab === "display" && <SettingsDisplayTab />}
            {selectedTab === "about" && <AboutTab />}
          </div>
        </div>
      </div>
    </CommonLayout>
  );
}
