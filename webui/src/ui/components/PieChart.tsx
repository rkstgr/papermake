"use client";
/*
 * Documentation:
 * Pie Chart — https://app.subframe.com/b658fa4250e4/library?component=Pie+Chart_0654ccc7-054c-4f3a-8e9a-b7c81dd3963c
 */

import React from "react";
import * as SubframeUtils from "../utils";
import * as SubframeCore from "@subframe/core";

interface PieChartRootProps
  extends React.ComponentProps<typeof SubframeCore.PieChart> {
  className?: string;
}

const PieChartRoot = React.forwardRef<HTMLElement, PieChartRootProps>(
  function PieChartRoot({ className, ...otherProps }: PieChartRootProps, ref) {
    return (
      <SubframeCore.PieChart
        className={SubframeUtils.twClassNames("h-52 w-52", className)}
        ref={ref as any}
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

export const PieChart = PieChartRoot;
