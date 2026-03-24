import {
  Body1,
  Caption1,
  Card,
  CardHeader,
  Switch,
} from "@fluentui/react-components";
import { emit } from "@tauri-apps/api/event";
import { Fragment, type ReactNode } from "react";

import { linkCard, linkChevron, winUiSwitch } from "./setting-item.css";

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

interface LinkSettingItemProps extends BaseSettingItemProps {
  type: "link";
  link?: string;
  onAction?: () => void;
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
  | LinkSettingItemProps
  | CustomSettingItemProps;

type SettingActionItemProps =
  | TextSettingItemProps
  | SwitchSettingItemProps
  | LinkSettingItemProps;

const OPEN_EXTERNAL_URL_EVENT = "waifudex://open-external-url";

function isExternalLink(link: string): boolean {
  return link.startsWith("http://") || link.startsWith("https://");
}

function ChevronRightIcon() {
  return (
    <svg
      className={linkChevron}
      width="16"
      height="16"
      viewBox="0 0 16 16"
      fill="currentColor"
      xmlns="http://www.w3.org/2000/svg"
    >
      <path d="M5.74 3.2a.75.75 0 0 0-.04 1.06L9.227 8 5.7 11.74a.75.75 0 1 0 1.1 1.02l4-4.25a.75.75 0 0 0 0-1.02l-4-4.25a.75.75 0 0 0-1.06-.04Z" />
    </svg>
  );
}

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

  if (props.type === "link") {
    return <ChevronRightIcon />;
  }

  return <Caption1>{props.value}</Caption1>;
}

function renderDescription(description?: string) {
  if (description === undefined) {
    return undefined;
  }

  return description
    .replaceAll("\\n", "\n")
    .split(/\r?\n/)
    .map((line, index) => (
      <Fragment key={`${line}-${index}`}>
        {index > 0 ? <br /> : null}
        {line}
      </Fragment>
    ));
}

function handleLinkClick(props: LinkSettingItemProps) {
  if (props.link && isExternalLink(props.link)) {
    void emit(OPEN_EXTERNAL_URL_EVENT, { url: props.link }).catch((error: unknown) =>
      console.error("openUrl failed:", error),
    );
    return;
  }

  if (props.onAction) {
    props.onAction();
  }
}

export default function SettingItem(props: SettingItemProps) {
  const { title, description } = props;
  const action = props.type ? <SettingActionItem {...props} /> : null;
  const isLink = props.type === "link";

  return (
    <Card
      appearance="filled"
      className={isLink ? linkCard : undefined}
      onClick={isLink ? () => handleLinkClick(props) : undefined}
    >
      <CardHeader
        header={<Body1>{title}</Body1>}
        description={<Caption1>{renderDescription(description)}</Caption1>}
        action={action}
      />
      {!props.type && props.children}
    </Card>
  );
}
