"use client";
/*
 * Documentation:
 * Sidebar rail with labels — https://app.subframe.com/b658fa4250e4/library?component=Sidebar+rail+with+labels_3296372a-ba83-4ca9-b291-10dc2aa86fdd
 * Avatar — https://app.subframe.com/b658fa4250e4/library?component=Avatar_bec25ae6-5010-4485-b46b-cf79e3943ab2
 */

import React from "react";
import * as SubframeUtils from "../utils";
import * as SubframeCore from "@subframe/core";
import { FeatherCircleDashed } from "@subframe/core";
import { Tooltip } from "./Tooltip";

interface NavItemProps extends React.HTMLAttributes<HTMLDivElement> {
  icon?: React.ReactNode;
  children?: React.ReactNode;
  selected?: boolean;
  className?: string;
}

const NavItem = React.forwardRef<HTMLElement, NavItemProps>(function NavItem(
  {
    icon = <FeatherCircleDashed />,
    children,
    selected = false,
    className,
    ...otherProps
  }: NavItemProps,
  ref
) {
  return (
    <SubframeCore.Tooltip.Provider>
      <SubframeCore.Tooltip.Root>
        <SubframeCore.Tooltip.Trigger asChild={true}>
          <div
            className={SubframeUtils.twClassNames(
              "group/8815d632 flex min-h-[48px] w-full cursor-pointer flex-col items-center justify-center gap-2 rounded-md px-2 pt-3 pb-2 active:bg-neutral-50",
              {
                "bg-neutral-100 hover:bg-neutral-100 active:bg-neutral-50":
                  selected,
              },
              className
            )}
            ref={ref as any}
            {...otherProps}
          >
            {icon ? (
              <SubframeCore.IconWrapper
                className={SubframeUtils.twClassNames(
                  "text-heading-2 font-heading-2 text-subtext-color group-hover/8815d632:text-default-font group-active/8815d632:text-default-font",
                  { "text-default-font": selected }
                )}
              >
                {icon}
              </SubframeCore.IconWrapper>
            ) : null}
            {children ? (
              <span
                className={SubframeUtils.twClassNames(
                  "line-clamp-1 w-full text-caption-bold font-caption-bold text-subtext-color text-center group-hover/8815d632:text-default-font group-active/8815d632:text-default-font",
                  { "text-default-font": selected }
                )}
              >
                {children}
              </span>
            ) : null}
          </div>
        </SubframeCore.Tooltip.Trigger>
        <SubframeCore.Tooltip.Portal>
          <SubframeCore.Tooltip.Content
            side="right"
            align="center"
            sideOffset={4}
            asChild={true}
          >
            <Tooltip>{children}</Tooltip>
          </SubframeCore.Tooltip.Content>
        </SubframeCore.Tooltip.Portal>
      </SubframeCore.Tooltip.Root>
    </SubframeCore.Tooltip.Provider>
  );
});

interface SidebarRailWithLabelsRootProps
  extends React.HTMLAttributes<HTMLElement> {
  header?: React.ReactNode;
  footer?: React.ReactNode;
  children?: React.ReactNode;
  className?: string;
}

const SidebarRailWithLabelsRoot = React.forwardRef<
  HTMLElement,
  SidebarRailWithLabelsRootProps
>(function SidebarRailWithLabelsRoot(
  {
    header,
    footer,
    children,
    className,
    ...otherProps
  }: SidebarRailWithLabelsRootProps,
  ref
) {
  return (
    <nav
      className={SubframeUtils.twClassNames(
        "flex h-full w-20 flex-col items-start bg-default-background",
        className
      )}
      ref={ref as any}
      {...otherProps}
    >
      {header ? (
        <div className="flex w-full flex-col items-center justify-center gap-2 px-6 py-6">
          {header}
        </div>
      ) : null}
      {children ? (
        <div className="flex w-full grow shrink-0 basis-0 flex-col items-center gap-1 px-2 py-2 overflow-auto">
          {children}
        </div>
      ) : null}
      {footer ? (
        <div className="flex w-full flex-col items-center justify-end gap-1 px-2 py-2">
          {footer}
        </div>
      ) : null}
    </nav>
  );
});

export const SidebarRailWithLabels = Object.assign(SidebarRailWithLabelsRoot, {
  NavItem,
});
