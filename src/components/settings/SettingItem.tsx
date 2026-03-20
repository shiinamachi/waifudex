import {
  Body1,
  Caption1,
  Card,
  CardHeader,
  Switch,
} from "@fluentui/react-components";

import { winUiSwitch } from "./setting-item.css";

type SettingItemType = "switch";

interface BaseSettingItemProps {
  title: string;
  description: string;
}

interface SwitchSettingItemProps extends BaseSettingItemProps {
  type: "switch";
  value: boolean;
  onChange: (newValue: boolean) => void;
}

type SettingItemProps = SwitchSettingItemProps;

type SettingActionItemProps = Pick<
  SettingItemProps,
  "type" | "value" | "onChange"
>;

function SettingActionItem({ type, value, onChange }: SettingActionItemProps) {
  if (type === "switch") {
    return (
      <Switch
        checked={value}
        className={winUiSwitch}
        onChange={(_event, data) => onChange(data.checked)}
      />
    );
  }

  return null;
}

export default function SettingItem({
  title,
  description,
  type,
  value,
  onChange,
}: SettingItemProps) {
  return (
    <Card appearance="filled">
      <CardHeader
        header={<Body1>{title}</Body1>}
        description={<Caption1>{description}</Caption1>}
        action={
          <SettingActionItem type={type} value={value} onChange={onChange} />
        }
      />
    </Card>
  );
}
