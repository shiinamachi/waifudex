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

import inventoryData from "../../../generated/dependencies.json";
import licenseTexts from "../../../licenses/license-texts.js";
import {
  accordionHeaderContent,
  accordionPanel,
  accordionPanelText,
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
  specifier?: string;
  license?: string[];
  repository?: string;
  author?: string;
}

interface InventorySection {
  label: string;
  manifestPath: string;
  roots: PackageEntry[];
  packages: PackageEntry[];
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
  const record = pkg as Record<string, string | string[] | undefined>;
  const entry: PackageEntry = {
    name: pkg.name,
    version: pkg.version,
  };
  if (typeof record.specifier === "string") entry.specifier = record.specifier;
  if (Array.isArray(record.license)) entry.license = record.license;
  if (typeof record.repository === "string")
    entry.repository = record.repository;
  if (typeof record.author === "string") entry.author = record.author;
  return entry;
}

function collectSections(): InventorySection[] {
  return [
    inventoryData.frontend,
    inventoryData.tauriClient,
    inventoryData.inochi2dSys,
    inventoryData.waifudexInox2dWasm,
    inventoryData.waifudexMascot,
  ].map((section) => ({
    label: section.label,
    manifestPath: section.manifestPath,
    roots: section.roots.map(toPackageEntry),
    packages: section.packages.map(toPackageEntry),
  }));
}

function normalizeSummaryToken(token: string): string | null {
  const normalized = token.replace(/^[()]+|[()]+$/g, "").trim();
  return normalized.length > 0 ? normalized : null;
}

function expandSummaryLicenses(licenses?: string[]): string[] {
  if (!licenses || licenses.length === 0) {
    return ["Unknown"];
  }

  const expanded = licenses
    .flatMap((license) => license.split(/\s+OR\s+|\s+AND\s+|\s*\/\s*/u))
    .map(normalizeSummaryToken)
    .filter((license): license is string => license !== null);

  return expanded.length > 0 ? Array.from(new Set(expanded)) : ["Unknown"];
}

function collectLicenseSummaries(packages: PackageEntry[]): LicenseSummary[] {
  const counts = new Map<string, number>();

  for (const pkg of packages) {
    for (const license of expandSummaryLicenses(pkg.license)) {
      counts.set(license, (counts.get(license) ?? 0) + 1);
    }
  }

  return Array.from(counts.entries())
    .map(([license, count]) => ({ license, count }))
    .sort((a, b) => b.count - a.count);
}

function formatLicenses(licenses?: string[]): string {
  if (!licenses || licenses.length === 0) {
    return "Unknown";
  }

  return licenses.join(" OR ");
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
  const metaParts: string[] = [formatLicenses(pkg.license)];
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

export default function OpenSourceLicenseTab({
  onBack,
}: OpenSourceLicenseTabProps) {
  const sections = useMemo(collectSections, []);
  const allPackages = useMemo(
    () => sections.flatMap((section) => section.packages),
    [sections],
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
              waifudex uses open source software. The following lists the
              licenses and packages used in this project.
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
                  <Badge
                    appearance="outline"
                    className={badgeStyle}
                    size="small"
                  >
                    {summary.count}
                  </Badge>
                </div>
              </AccordionHeader>
              <AccordionPanel className={accordionPanel}>
                <pre className={accordionPanelText}>
                  {licenseTexts[summary.license] ?? licenseTexts.Unknown}
                </pre>
              </AccordionPanel>
            </AccordionItem>
          ))}
        </Accordion>
      </section>

      {sections.map((inventorySection) => (
        <section className={section} key={inventorySection.label}>
          <div className={sectionHeader}>
            <Subtitle2 className={sectionTitle}>
              {inventorySection.label}
            </Subtitle2>
            <Caption1 className={sectionMeta}>
              {inventorySection.packages.length} packages
            </Caption1>
          </div>
          <div className={packageList}>
            {inventorySection.packages.map((pkg) => (
              <PackageCardItem
                key={`${inventorySection.label}:${pkg.name}@${pkg.version}`}
                pkg={pkg}
              />
            ))}
          </div>
        </section>
      ))}
    </div>
  );
}
