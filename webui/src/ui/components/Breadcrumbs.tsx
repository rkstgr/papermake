"use client";
/*
 * Documentation:
 * Breadcrumbs — https://app.subframe.com/b658fa4250e4/library?component=Breadcrumbs_8898334b-a66f-4ee8-8bd1-afcfa8e37cc0
 */

import React from "react";
import * as SubframeUtils from "../utils";
import { FeatherChevronRight } from "@subframe/core";

interface ItemProps extends React.HTMLAttributes<HTMLSpanElement> {
  children?: React.ReactNode;
  active?: boolean;
  className?: string;
}

const Item = React.forwardRef<HTMLElement, ItemProps>(function Item(
  { children, active = false, className, ...otherProps }: ItemProps,
  ref
) {
  return children ? (
    <span
      className={SubframeUtils.twClassNames(
        "group/bbdc1640 line-clamp-1 cursor-pointer break-words text-body font-body text-subtext-color hover:text-default-font",
        { "text-default-font": active },
        className
      )}
      ref={ref as any}
      {...otherProps}
    >
      {children}
    </span>
  ) : null;
});

interface DividerProps
  extends React.ComponentProps<typeof FeatherChevronRight> {
  className?: string;
}

const Divider = React.forwardRef<HTMLElement, DividerProps>(function Divider(
  { className, ...otherProps }: DividerProps,
  ref
) {
  return (
    <FeatherChevronRight
      className={SubframeUtils.twClassNames(
        "text-body font-body text-subtext-color",
        className
      )}
      ref={ref as any}
      {...otherProps}
    />
  );
});

interface BreadcrumbsRootProps extends React.HTMLAttributes<HTMLDivElement> {
  children?: React.ReactNode;
  className?: string;
}

const BreadcrumbsRoot = React.forwardRef<HTMLElement, BreadcrumbsRootProps>(
  function BreadcrumbsRoot(
    { children, className, ...otherProps }: BreadcrumbsRootProps,
    ref
  ) {
    return children ? (
      <div
        className={SubframeUtils.twClassNames(
          "flex items-center gap-2",
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

export const Breadcrumbs = Object.assign(BreadcrumbsRoot, {
  Item,
  Divider,
});
