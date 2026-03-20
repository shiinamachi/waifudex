import {
  Body1,
  Caption1,
  Card,
  CardHeader,
  Switch,
} from "@fluentui/react-components";
import type { ReactNode } from "react";

import { winUiSwitch } from "./setting-item.css";

interface BaseSettingItemProps {
  title: string;
  description: string;
}

interface SwitchSettingItemProps extends BaseSettingItemProps {
  type: "switch";
  value: boolean;
  onChange: (newValue: boolean) => void;
  children?: never;
}

interface CustomSettingItemProps extends BaseSettingItemProps {
  type?: never;
  value?: never;
  onChange?: never;
  children: ReactNode;
}

type SettingItemProps = SwitchSettingItemProps | CustomSettingItemProps;

type SettingActionItemProps = Pick<
  SwitchSettingItemProps,
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

export default function SettingItem(props: SettingItemProps) {
  const { title, description } = props;

  return (
    <Card appearance="filled">
      <CardHeader
        header={<Body1>{title}</Body1>}
        description={<Caption1>{description}</Caption1>}
        action={
          props.type === "switch" ? (
            <SettingActionItem
              type={props.type}
              value={props.value}
              onChange={props.onChange}
            />
          ) : undefined
        }
      />
      {!props.type && props.children}
    </Card>
  );
}
