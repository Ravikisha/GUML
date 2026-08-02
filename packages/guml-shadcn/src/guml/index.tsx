"use client";

/**
 * GUML-convention wrappers over the shadcn components.
 *
 * WHY THIS FILE EXISTS, because it is the whole reason a registry package is a package and not a JSON file:
 *
 * The compiler emits one shape for every `field`-kind tag, whoever wrote it —
 *
 *     <Slider value={volume} onChange={setVolume} min={0} max={100} aria-label="Volume" />
 *
 * `onChange` takes **the value**, not an event, and `value` is the scalar the state holds. That uniformity is
 * what lets one lowering serve every field in the vocabulary, builtin or contributed.
 *
 * shadcn's components each carry their upstream primitive's API instead. Radix's Slider is `number[]` and
 * `onValueChange`; a raw `<textarea>` is a React `ChangeEvent`; Base UI's Combobox is a compound of six
 * elements. All three are correct for their own library and none of them is the shape above.
 *
 * Something has to reconcile the two, and there are only three places it could live:
 *
 *   1. In the compiler, as a per-component table of prop spellings. That is a copy of shadcn's API inside
 *      GUML, wrong the day shadcn changes, and it would have to grow a branch for every package anyone ever
 *      writes.
 *   2. In the registry JSON, as a mapping language. Reinventing adapters in a format that cannot express
 *      `number[]`, let alone a compound.
 *   3. Here, in the package, in the language the components are written in.
 *
 * Three is the only one that scales, and it is why `element`/`import` point at a *component* rather than at a
 * DOM tag: the host owns the glue. Everything below is that glue and nothing else — no styling, no new
 * behaviour. The real components stay untouched in `components/ui/`, so `shadcn add <name>` still updates
 * them in place.
 */

import * as React from "react";

import { Button } from "../components/ui/button";
import { Calendar } from "../components/ui/calendar";
import {
  Collapsible as CollapsibleRoot,
  CollapsibleContent,
  CollapsibleTrigger,
} from "../components/ui/collapsible";
import {
  Combobox as ComboboxRoot,
  ComboboxContent,
  ComboboxEmpty,
  ComboboxInput,
  ComboboxItem,
  ComboboxList,
} from "../components/ui/combobox";
import {
  InputOTP as InputOTPRoot,
  InputOTPGroup,
  InputOTPSlot,
} from "../components/ui/input-otp";
import { Label } from "../components/ui/label";
import { Popover, PopoverContent, PopoverTrigger } from "../components/ui/popover";
import {
  RadioGroup as RadioGroupRoot,
  RadioGroupItem,
} from "../components/ui/radio-group";
import { Slider as SliderRoot } from "../components/ui/slider";
import { Textarea as TextareaRaw } from "../components/ui/textarea";
import {
  ToggleGroup as ToggleGroupRoot,
  ToggleGroupItem,
} from "../components/ui/toggle-group";
import { cn } from "../lib/utils";

/**
 * What the compiler emits for every `field`-kind tag. `options` is present only when the bound state has a
 * domain or the element was written with `option` children — a choice among alternatives needs the
 * alternatives, and both spellings reduce to this one list.
 */
export type GumlField<T> = {
  value: T;
  onChange: (next: T) => void;
  options?: readonly string[];
  "aria-label"?: string;
  className?: string;
  disabled?: boolean;
};

/** A stable id per control, so a generated label can point at the input it names. */
function useFieldId(explicit?: string) {
  const auto = React.useId();
  return explicit ?? auto;
}

/* -------------------------------------------------------------------------- */
/* fields                                                                      */
/* -------------------------------------------------------------------------- */

/** Multi-line text. The only difference from the raw component is unwrapping the change event. */
export function Textarea({
  value,
  onChange,
  options: _options,
  ...rest
}: GumlField<string> & Omit<React.ComponentProps<"textarea">, "value" | "onChange">) {
  return (
    <TextareaRaw {...rest} value={value} onChange={(e) => onChange(e.target.value)} />
  );
}

/**
 * One-of-many. Radix needs an item per option and a visible label per item, which is why `options` had to
 * reach the component at all: a `RadioGroup` with no children is a control the reader cannot operate.
 */
export function RadioGroup<T extends string>({
  value,
  onChange,
  options = [],
  className,
  ...rest
}: GumlField<T>) {
  const id = useFieldId();
  return (
    <RadioGroupRoot
      {...rest}
      className={cn("gap-3", className)}
      value={value}
      onValueChange={(next) => onChange(next as T)}
    >
      {options.map((option) => (
        <div key={option} className="flex items-center gap-2">
          <RadioGroupItem id={`${id}-${option}`} value={option} />
          <Label htmlFor={`${id}-${option}`}>{option}</Label>
        </div>
      ))}
    </RadioGroupRoot>
  );
}

/** A numeric range. Radix models the thumbs as an array; GUML binds a single number. */
export function Slider({
  value,
  onChange,
  options: _options,
  ...rest
}: GumlField<number> & { min?: number; max?: number; step?: number }) {
  return (
    <SliderRoot
      {...rest}
      value={[value]}
      onValueChange={([next]) => onChange(next ?? value)}
    />
  );
}

/** A fixed-length code. `length` is GUML's spelling of `maxLength`. */
export function InputOTP({
  value,
  onChange,
  options: _options,
  length = 6,
  ...rest
}: GumlField<string> & { length?: number }) {
  return (
    <InputOTPRoot {...rest} maxLength={length} value={value} onChange={onChange}>
      <InputOTPGroup>
        {Array.from({ length }, (_, i) => (
          <InputOTPSlot key={i} index={i} />
        ))}
      </InputOTPGroup>
    </InputOTPRoot>
  );
}

/** Filterable single-select. Base UI models it as a compound; GUML binds one value. */
export function Combobox<T extends string>({
  value,
  onChange,
  options = [],
  placeholder,
  ...rest
}: GumlField<T> & { placeholder?: string }) {
  return (
    <ComboboxRoot
      {...rest}
      items={options as string[]}
      value={value}
      onValueChange={(next: string | null) => onChange((next ?? "") as T)}
    >
      <ComboboxInput placeholder={placeholder} />
      <ComboboxContent>
        <ComboboxEmpty>No match.</ComboboxEmpty>
        <ComboboxList>
          {options.map((option) => (
            <ComboboxItem key={option} value={option}>
              {option}
            </ComboboxItem>
          ))}
        </ComboboxList>
      </ComboboxContent>
    </ComboboxRoot>
  );
}

/**
 * A date, bound as an ISO `YYYY-MM-DD` string.
 *
 * shadcn ships no `DatePicker` component — its date-picker page is a *recipe* composing Popover, Button and
 * Calendar, so there is no `date-picker.tsx` to install and this is the composition written once. It is the
 * one entry here that is a real component rather than a prop rename, and the registry declared it before it
 * existed; the typecheck gate is what caught that.
 *
 * ISO in, ISO out, and parsed as UTC. `new Date("2026-03-01")` is midnight UTC but `new Date(2026, 2, 1)` is
 * midnight local, so round-tripping through the local-time constructor moves the date by a day for anyone
 * west of Greenwich.
 */
export function DatePicker({
  value,
  onChange,
  options: _options,
  min,
  max,
  ...rest
}: GumlField<string> & { min?: string; max?: string }) {
  const [open, setOpen] = React.useState(false);
  const selected = value ? new Date(`${value}T00:00:00Z`) : undefined;
  const valid = selected && !Number.isNaN(selected.getTime()) ? selected : undefined;

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
            variant="outline"
            aria-label={rest["aria-label"]}
            disabled={rest.disabled}
            className={cn(
              "w-full justify-start font-normal",
              !valid && "text-muted-foreground",
              rest.className,
            )}
          >
          {valid ? valid.toISOString().slice(0, 10) : "Pick a date"}
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-auto p-0" align="start">
        <Calendar
          mode="single"
          selected={valid}
          defaultMonth={valid}
          startMonth={min ? new Date(`${min}T00:00:00Z`) : undefined}
          endMonth={max ? new Date(`${max}T00:00:00Z`) : undefined}
          onSelect={(next?: Date) => {
            onChange(next ? next.toISOString().slice(0, 10) : "");
            setOpen(false);
          }}
        />
      </PopoverContent>
    </Popover>
  );
}

/** Mutually exclusive toggles. `multiple` switches Radix's `type`, which also changes the value's shape. */
export function ToggleGroup<T extends string>({
  value,
  onChange,
  options = [],
  multiple = false,
  ...rest
}: GumlField<T> & { multiple?: boolean }) {
  const items = options.map((option) => (
    <ToggleGroupItem key={option} value={option}>
      {option}
    </ToggleGroupItem>
  ));

  // Not one element with a computed `type`: Radix's single and multiple roots take different value types, and
  // a union of the two is not assignable to either.
  return multiple ? (
    <ToggleGroupRoot
      {...rest}
      type="multiple"
      value={value ? value.split(",") : []}
      onValueChange={(next: string[]) => onChange(next.join(",") as T)}
    >
      {items}
    </ToggleGroupRoot>
  ) : (
    <ToggleGroupRoot
      {...rest}
      type="single"
      value={value}
      onValueChange={(next: string) => onChange(next as T)}
    >
      {items}
    </ToggleGroupRoot>
  );
}

/* -------------------------------------------------------------------------- */
/* containers                                                                  */
/* -------------------------------------------------------------------------- */

/**
 * A section that expands. Radix wants a controlled boolean and a trigger element; GUML writes
 * `collapsible "Advanced" open` and the summary reaches us as `aria-label`, since the tag declares
 * `requires_label`.
 *
 * `open` is the *initial* state — GUML's attribute is a starting condition, not a binding, so the component
 * owns the state afterwards. Accepting a number too because `open=1` is how the attribute is written when a
 * document reuses the `faq` convention, and coercing is better than a type error on a document that reads
 * exactly as intended.
 */
export function Collapsible({
  open = false,
  children,
  className,
  ...rest
}: Omit<React.ComponentProps<typeof CollapsibleRoot>, "open"> & {
  open?: boolean | number;
}) {
  const [isOpen, setIsOpen] = React.useState(Boolean(open));
  const label = rest["aria-label"];

  return (
    <CollapsibleRoot
      {...rest}
      open={isOpen}
      onOpenChange={setIsOpen}
      className={cn("w-full", className)}
    >
      <CollapsibleTrigger asChild>
        <Button variant="ghost" size="sm" className="w-full justify-between">
          {label ?? "Details"}
          <span aria-hidden="true">{isOpen ? "−" : "+"}</span>
        </Button>
      </CollapsibleTrigger>
      <CollapsibleContent className="pt-2">{children}</CollapsibleContent>
    </CollapsibleRoot>
  );
}
