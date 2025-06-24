"use client";
/*
 * Documentation:
 * Checkbox Group — https://app.subframe.com/b658fa4250e4/library?component=Checkbox+Group_f9f1b596-c6b3-4d60-aa9a-f34b353f8aa5
 * Checkbox — https://app.subframe.com/b658fa4250e4/library?component=Checkbox_3816e3b5-c48c-499b-b45e-0777c6972523
 */

import React from "react";
import * as SubframeUtils from "../utils";

interface CheckboxGroupRootProps extends React.HTMLAttributes<HTMLDivElement> {
  label?: React.ReactNode;
  helpText?: React.ReactNode;
  error?: boolean;
  horizontal?: boolean;
  children?: React.ReactNode;
  className?: string;
}

const CheckboxGroupRoot = React.forwardRef<HTMLElement, CheckboxGroupRootProps>(
  function CheckboxGroupRoot(
    {
      label,
      helpText,
      error = false,
      horizontal = false,
      children,
      className,
      ...otherProps
    }: CheckboxGroupRootProps,
    ref
  ) {
    return (
      <div
        className={SubframeUtils.twClassNames(
          "group/f9f1b596 flex flex-col items-start gap-2",
          { "flex-col flex-nowrap gap-2": horizontal },
          className
        )}
        ref={ref as any}
        {...otherProps}
      >
        {label ? (
          <span
            className={SubframeUtils.twClassNames(
              "text-body-bold font-body-bold text-default-font",
              { "text-default-font": error }
            )}
          >
            {label}
          </span>
        ) : null}
        {children ? (
          <div
            className={SubframeUtils.twClassNames(
              "flex flex-col items-start gap-2",
              { "flex-row flex-nowrap gap-6": horizontal }
            )}
          >
            {children}
          </div>
        ) : null}
        {helpText ? (
          <span
            className={SubframeUtils.twClassNames(
              "text-caption font-caption text-subtext-color",
              { "text-error-700": error }
            )}
          >
            {helpText}
          </span>
        ) : null}
      </div>
    );
  }
);

export const CheckboxGroup = CheckboxGroupRoot;
