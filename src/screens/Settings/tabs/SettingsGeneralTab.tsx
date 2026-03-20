import SettingItem from "../../../components/settings/SettingItem";
import { useAppSetting } from "../../../hooks/useAppSetting";

export default function SettingsGeneralTab() {
  const { value: alwaysOnTop, setValue } = useAppSetting("alwaysOnTop");

  return (
    <div>
      <SettingItem
        title="Always on top"
        description="Always display the waifudex character at the top"
        type="switch"
        value={alwaysOnTop}
        onChange={setValue}
      />
    </div>
  );
}
