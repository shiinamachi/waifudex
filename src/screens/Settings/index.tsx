import { useState } from "react";
import {
  Tab,
  TabList,
  type SelectTabData,
  type SelectTabEvent,
  type TabValue,
} from "@fluentui/react-components";

import CommonLayout from "../../components/layouts/CommonLayout";
import { layout, panel, tabList } from "./index.css";

const DEFAULT_TAB: TabValue = "general";

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
          <Tab value="general">General</Tab>
        </TabList>

        <div className={panel}>
          {selectedTab === "general" && "General Settings"}
        </div>
      </div>
    </CommonLayout>
  );
}
