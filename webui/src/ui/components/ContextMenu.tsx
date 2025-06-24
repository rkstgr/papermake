"use client";
/*
 * Documentation:
 * Context Menu — https://app.subframe.com/b658fa4250e4/library?component=Context+Menu_f8a49f07-fa5b-46c8-9399-cbbf0930cc62
 * Badge — https://app.subframe.com/b658fa4250e4/library?component=Badge_97bdb082-1124-4dd7-a335-b14b822d0157
 */

import React from "react";
import * as SubframeUtils from "../utils";
import * as SubframeCore from "@subframe/core";
import { FeatherStar } from "@subframe/core";

interface ContextItemProps
  extends React.ComponentProps<typeof SubframeCore.ContextMenu.Item> {
  children?: React.ReactNode;
  icon?: React.ReactNode;
  rightSlot?: React.ReactNode;
  className?: string;
}

const ContextItem = React.forwardRef<HTMLElement, ContextItemProps>(
  function ContextItem(
    {
      children,
      icon = <FeatherStar />,
      rightSlot,
      className,
      ...otherProps
    }: ContextItemProps,
    ref
  ) {
    return (
      <SubframeCore.ContextMenu.Item asChild={true} {...otherProps}>
        <div
          className={SubframeUtils.twClassNames(
            "group/9358ee78 flex h-8 w-full cursor-pointer items-center gap-2 rounded-md px-2 hover:bg-neutral-100 active:bg-neutral-50 data-[highlighted]:bg-neutral-100",
            className
          )}
          ref={ref as any}
        >
          <div className="flex h-4 w-4 flex-none items-center justify-center gap-2">
            {icon ? (
              <SubframeCore.IconWrapper className="text-body font-body text-default-font">
                {icon}
              </SubframeCore.IconWrapper>
            ) : null}
          </div>
          {children ? (
            <span className="line-clamp-1 grow shrink-0 basis-0 text-body font-body text-default-font group-hover/9358ee78:text-default-font">
              {children}
            </span>
          ) : null}
          {rightSlot ? (
            <div className="flex flex-col items-end justify-center gap-2">
              {rightSlot}
            </div>
          ) : null}
        </div>
      </SubframeCore.ContextMenu.Item>
    );
  }
);

interface ContextDividerProps extends React.HTMLAttributes<HTMLDivElement> {
  className?: string;
}

const ContextDivider = React.forwardRef<HTMLElement, ContextDividerProps>(
  function ContextDivider(
    { className, ...otherProps }: ContextDividerProps,
    ref
  ) {
    return (
      <div
        className={SubframeUtils.twClassNames(
          "flex w-full items-start gap-2 px-1 py-1",
          className
        )}
        ref={ref as any}
        {...otherProps}
      >
        <div className="flex h-px grow shrink-0 basis-0 flex-col items-center gap-2 bg-neutral-200" />
      </div>
    );
  }
);

interface ContextMenuRootProps extends React.HTMLAttributes<HTMLDivElement> {
  children?: React.ReactNode;
  className?: string;
}

const ContextMenuRoot = React.forwardRef<HTMLElement, ContextMenuRootProps>(
  function ContextMenuRoot(
    { children, className, ...otherProps }: ContextMenuRootProps,
    ref
  ) {
    return children ? (
      <div
        className={SubframeUtils.twClassNames(
          "flex min-w-[192px] flex-col items-start rounded-md border border-solid border-neutral-border bg-default-background px-1 py-1 shadow-lg",
          className
        )}
        ref={ref as any}
        {...otherProps}
      >
        {children}
      </div>
    ) : null;
  }
);

export const ContextMenu = Object.assign(ContextMenuRoot, {
  ContextItem,
  ContextDivider,
});
