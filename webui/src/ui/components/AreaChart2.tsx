"use client";
/*
 * Documentation:
 * Area Chart2 — https://app.subframe.com/b658fa4250e4/library?component=Area+Chart2_b615ef4c-161c-4a06-86e1-d6d90a0b8d4a
 */

import React from "react";
import * as SubframeUtils from "../utils";
import * as SubframeCore from "@subframe/core";

interface AreaChart2RootProps
  extends React.ComponentProps<typeof SubframeCore.AreaChart> {
  stacked?: boolean;
  className?: string;
}

const AreaChart2Root = React.forwardRef<HTMLElement, AreaChart2RootProps>(
  function AreaChart2Root(
    { stacked = false, className, ...otherProps }: AreaChart2RootProps,
    ref
  ) {
    return (
      <SubframeCore.AreaChart
        className={SubframeUtils.twClassNames("h-80 w-full", className)}
        ref={ref as any}
        stacked={stacked}
        colors={[
          "#6366f1",
          "#c7d2fe",
          "#4f46e5",
          "#a5b4fc",
          "#4338ca",
          "#818cf8",
        ]}
        {...otherProps}
      />
    );
  }
);

export const AreaChart2 = AreaChart2Root;
