"use client";
/*
 * Documentation:
 * Calendar — https://app.subframe.com/b658fa4250e4/library?component=Calendar_5a87e517-ace2-49af-adcf-076c97ec3921
 */

import React from "react";
import * as SubframeUtils from "../utils";
import * as SubframeCore from "@subframe/core";

type CalendarRootProps = React.ComponentProps<typeof SubframeCore.Calendar> & {
  className?: string;
};

const CalendarRoot = React.forwardRef<HTMLElement, CalendarRootProps>(
  function CalendarRoot({ className, ...otherProps }: CalendarRootProps, ref) {
    return (
      <SubframeCore.Calendar
        className={className}
        ref={ref as any}
        classNames={{
          root: "relative border-box",
          month: "flex flex-col gap-4",
          months: "relative flex flex-wrap max-w-fit gap-4",
          nav: "absolute flex items-center justify-between h-8 w-full p-0.5",
          month_caption: "flex items-center justify-center h-8",
          caption_label: "text-body-bold font-body-bold text-default-font",
          button_previous:
            "inline-flex items-center justify-center h-8 w-8 bg-transparent rounded border-none bg-transparent hover:bg-neutral-50 active:bg-neutral-100 border-none",
          button_next:
            "inline-flex items-center justify-center h-8 w-8 bg-transparent rounded border-none bg-transparent hover:bg-neutral-50 active:bg-neutral-100 border-none",
          chevron: "text-[18px] font-[500] leading-[18px] text-neutral-600",
          weeks: "flex flex-col gap-2",
          weekdays: "flex pb-4",
          weekday: "w-8 text-caption-bold font-caption-bold text-subtext-color",
          week: "flex rounded-lg overflow-hidden",
          day: "group flex p-0 cursor-pointer items-center justify-center text-body font-body text-default-font h-8 w-8",
          day_button:
            "flex h-8 w-8 cursor-pointer items-center justify-center gap-2 rounded-lg border-none hover:bg-neutral-100 group-[.selected]:bg-brand-600 group-[.selected]:text-white group-[.outside]:bg-transparent group-[.outside.selected]:bg-neutral-100 group-[.outside.range-start]:bg-neutral-100 group-[.outside.range-end]:bg-neutral-100 group-[.outside.range-start]:rounded-l-lg group-[.outside.range-start]:rounded-r-none group-[.outside.range-end]:rounded-l-none group-[.outside.range-end]:rounded-r-lg group-[.outside]:!text-neutral-400 group-[.outside]:hover:bg-neutral-100 group-[.range-middle.selected]:bg-neutral-100 group-[.range-middle.selected]:text-default-font group-[.range-middle.selected]:hover:bg-neutral-100 group-[.range-middle.selected]:rounded-none",
          selected: "selected",
          outside: "outside",
          range_start: "range-start bg-neutral-100 rounded-l-lg",
          range_middle: "range-middle",
          range_end: "range-end bg-neutral-100 rounded-r-lg",
        }}
        {...otherProps}
      />
    );
  }
);

export const Calendar = CalendarRoot;
