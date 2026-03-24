import {
  Body1,
  Caption1,
  Card,
  CardHeader,
  Switch,
} from "@fluentui/react-components";
import { Fragment, type ReactNode } from "react";

import { winUiSwitch } from "./setting-item.css";

interface BaseSettingItemProps {
  title: string;
  description?: string;
}

interface TextSettingItemProps extends BaseSettingItemProps {
  type: "text";
  value: string;
  children?: never;
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
  children?: ReactNode;
}

type SettingItemProps =
  | TextSettingItemProps
  | SwitchSettingItemProps
  | CustomSettingItemProps;

type SettingActionItemProps = TextSettingItemProps | SwitchSettingItemProps;

function SettingActionItem(props: SettingActionItemProps) {
  if (props.type === "switch") {
    return (
      <Switch
        checked={props.value}
        className={winUiSwitch}
        onChange={(_event, data) => props.onChange(data.checked)}
      />
    );
  }

  return <Caption1>{props.value}</Caption1>;
}

function renderDescription(description?: string) {
  if (description === undefined) {
    return undefined;
  }

  return description.replaceAll("\\n", "\n").split(/\r?\n/).map((line, index) => (
    <Fragment key={`${line}-${index}`}>
      {index > 0 ? <br /> : null}
      {line}
    </Fragment>
  ));
}

export default function SettingItem(props: SettingItemProps) {
  const { title, description } = props;
  const action = props.type ? <SettingActionItem {...props} /> : null;

  return (
    <Card appearance="filled">
      <CardHeader
        header={<Body1>{title}</Body1>}
        description={<Caption1>{renderDescription(description)}</Caption1>}
        action={action}
      />
      {!props.type && props.children}
    </Card>
  );
}
