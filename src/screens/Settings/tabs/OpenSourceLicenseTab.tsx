import {
  Accordion,
  AccordionHeader,
  AccordionItem,
  AccordionPanel,
  Badge,
  Body1,
  Button,
  Caption1,
  Card,
  CardHeader,
  Subtitle2,
} from "@fluentui/react-components";
import { emit } from "@tauri-apps/api/event";
import React, { useMemo } from "react";

import inventoryData from "../../../../waifudex-dependency-inventory.json";
import {
  accordionHeaderContent,
  accordionPanel,
  badgeStyle,
  container,
  header,
  headerContent,
  notice,
  packageCard,
  packageHeader,
  packageList,
  packageMeta,
  pageTitle,
  repositoryLink,
  section,
  sectionHeader,
  sectionMeta,
  sectionTitle,
  stickyHeader,
  summaryAccordion,
} from "./open-source-license-tab.css";

interface PackageEntry {
  name: string;
  version: string;
  license?: string;
  repository?: string;
  author?: string;
}

interface LicenseSummary {
  license: string;
  count: number;
}

interface OpenSourceLicenseTabProps {
  onBack: () => void;
}

const OPEN_EXTERNAL_URL_EVENT = "waifudex://open-external-url";

function toPackageEntry(pkg: { name: string; version: string }): PackageEntry {
  const record = pkg as Record<string, string | undefined>;
  const entry: PackageEntry = {
    name: pkg.name,
    version: pkg.version,
  };
  if (record.license !== undefined) entry.license = record.license;
  if (record.repository !== undefined) entry.repository = record.repository;
  if (record.author !== undefined) entry.author = record.author;
  return entry;
}

function collectJsPackages(): PackageEntry[] {
  return inventoryData.javascript.locked_packages.map(toPackageEntry);
}

function collectRustPackages(): PackageEntry[] {
  const seen = new Set<string>();
  const packages: PackageEntry[] = [];

  for (const entry of inventoryData.rust) {
    for (const pkg of entry.locked_packages) {
      const key = `${pkg.name}@${pkg.version}`;
      if (!seen.has(key)) {
        seen.add(key);
        packages.push(toPackageEntry(pkg));
      }
    }
  }

  return packages;
}

function collectLicenseSummaries(packages: PackageEntry[]): LicenseSummary[] {
  const counts = new Map<string, number>();

  for (const pkg of packages) {
    const license = pkg.license ?? "Unknown";
    counts.set(license, (counts.get(license) ?? 0) + 1);
  }

  return Array.from(counts.entries())
    .map(([license, count]) => ({ license, count }))
    .sort((a, b) => b.count - a.count);
}

function openLink(url: string) {
  void emit(OPEN_EXTERNAL_URL_EVENT, { url }).catch((error: unknown) =>
    console.error("openUrl failed:", error),
  );
}

function ChevronLeftIcon() {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 16 16"
      fill="currentColor"
      xmlns="http://www.w3.org/2000/svg"
    >
      <path d="M10.26 3.2a.75.75 0 0 1 .04 1.06L6.773 8l3.527 3.74a.75.75 0 1 1-1.1 1.02l-4-4.25a.75.75 0 0 1 0-1.02l4-4.25a.75.75 0 0 1 1.06-.04Z" />
    </svg>
  );
}

function PackageCardItem({ pkg }: { pkg: PackageEntry }) {
  const license = pkg.license ?? "Unknown";
  const metaParts: string[] = [license];
  if (pkg.author) {
    metaParts.push(pkg.author);
  }

  return (
    <Card appearance="filled" className={packageCard}>
      <CardHeader
        header={
          <div className={packageHeader}>
            <Body1>{pkg.name}</Body1>
            <Caption1>{pkg.version}</Caption1>
          </div>
        }
        description={
          <div className={packageMeta}>
            <Caption1>{metaParts.join(" · ")}</Caption1>
            {pkg.repository && (
              <>
                <Caption1>·</Caption1>
                <Caption1
                  className={repositoryLink}
                  onClick={(event: React.MouseEvent) => {
                    event.stopPropagation();
                    openLink(pkg.repository!);
                  }}
                >
                  Repository
                </Caption1>
              </>
            )}
          </div>
        }
      />
    </Card>
  );
}

export default function OpenSourceLicenseTab({ onBack }: OpenSourceLicenseTabProps) {
  const jsPackages = useMemo(collectJsPackages, []);
  const rustPackages = useMemo(collectRustPackages, []);
  const allPackages = useMemo(
    () => [...jsPackages, ...rustPackages],
    [jsPackages, rustPackages],
  );
  const licenseSummaries = useMemo(
    () => collectLicenseSummaries(allPackages),
    [allPackages],
  );

  return (
    <div className={container}>
      <div className={stickyHeader}>
        <div className={header}>
          <Button
            appearance="subtle"
            icon={<ChevronLeftIcon />}
            onClick={onBack}
          />
          <div className={headerContent}>
            <Subtitle2 className={pageTitle}>Open Source License</Subtitle2>
            <Caption1 className={notice}>
              This application uses open source software. The following lists
              the licenses and packages used in this project.
            </Caption1>
          </div>
        </div>
      </div>

      <section className={section}>
        <div className={sectionHeader}>
          <Subtitle2 className={sectionTitle}>License Summary</Subtitle2>
          <Caption1 className={sectionMeta}>
            {licenseSummaries.length} licenses
          </Caption1>
        </div>
        <Accordion className={summaryAccordion} collapsible multiple>
          {licenseSummaries.map((summary) => (
            <AccordionItem key={summary.license} value={summary.license}>
              <AccordionHeader>
                <div className={accordionHeaderContent}>
                  <span>{summary.license}</span>
                  <Badge appearance="outline" className={badgeStyle} size="small">
                    {summary.count}
                  </Badge>
                </div>
              </AccordionHeader>
              <AccordionPanel className={accordionPanel}>
                <Caption1>
                  License text will be added in a future update.
                </Caption1>
              </AccordionPanel>
            </AccordionItem>
          ))}
        </Accordion>
      </section>

      <section className={section}>
        <div className={sectionHeader}>
          <Subtitle2 className={sectionTitle}>JavaScript</Subtitle2>
          <Caption1 className={sectionMeta}>{jsPackages.length} packages</Caption1>
        </div>
        <div className={packageList}>
          {jsPackages.map((pkg) => (
            <PackageCardItem key={`${pkg.name}@${pkg.version}`} pkg={pkg} />
          ))}
        </div>
      </section>

      <section className={section}>
        <div className={sectionHeader}>
          <Subtitle2 className={sectionTitle}>Rust</Subtitle2>
          <Caption1 className={sectionMeta}>{rustPackages.length} packages</Caption1>
        </div>
        <div className={packageList}>
          {rustPackages.map((pkg) => (
            <PackageCardItem key={`${pkg.name}@${pkg.version}`} pkg={pkg} />
          ))}
        </div>
      </section>
    </div>
  );
}
