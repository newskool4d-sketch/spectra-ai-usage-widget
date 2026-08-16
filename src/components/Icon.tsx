import { memo, type ReactNode } from "react";

export type IconName = "grid" | "list" | "pulse" | "spark" | "bell" | "settings" | "refresh" | "shield" | "phone" | "monitor" | "eye" | "chevron" | "check" | "link" | "x";

const iconShapes: Readonly<Record<IconName, ReactNode>> = {
  grid: <><rect x="3" y="3" width="7" height="7" rx="2" /><rect x="14" y="3" width="7" height="7" rx="2" /><rect x="3" y="14" width="7" height="7" rx="2" /><rect x="14" y="14" width="7" height="7" rx="2" /></>,
  list: <><path d="M9 6h12M9 12h12M9 18h12" /><circle cx="4.5" cy="6" r="1.2" /><circle cx="4.5" cy="12" r="1.2" /><circle cx="4.5" cy="18" r="1.2" /></>,
  pulse: <path d="M3 12h4l2.2-6 4.1 12 2.2-6H21" />,
  spark: <><path d="m4 17 5-5 3 3 7-8" /><path d="M15 7h4v4" /></>,
  bell: <><path d="M18 8a6 6 0 0 0-12 0c0 7-3 7-3 9h18c0-2-3-2-3-9" /><path d="M10 21h4" /></>,
  settings: <><path d="M12 15.5a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7Z" /><path d="M19.4 15a1.7 1.7 0 0 0 .34 1.88l.06.06-2.83 2.83-.06-.06A1.7 1.7 0 0 0 15 19.4a1.7 1.7 0 0 0-1 .6 1.7 1.7 0 0 0-.4 1.1V21h-4v-.09A1.7 1.7 0 0 0 8.6 19.4a1.7 1.7 0 0 0-1.88.34l-.06.06-2.83-2.83.06-.06A1.7 1.7 0 0 0 4.6 15a1.7 1.7 0 0 0-.6-1 1.7 1.7 0 0 0-1.1-.4H3v-4h.09A1.7 1.7 0 0 0 4.6 8.6a1.7 1.7 0 0 0-.34-1.88l-.06-.06 2.83-2.83.06.06A1.7 1.7 0 0 0 9 4.6a1.7 1.7 0 0 0 1-.6 1.7 1.7 0 0 0 .4-1.1V3h4v.09A1.7 1.7 0 0 0 15.4 4.6a1.7 1.7 0 0 0 1.88-.34l.06-.06 2.83 2.83-.06.06A1.7 1.7 0 0 0 19.4 9c.14.37.35.7.6 1 .3.3.7.48 1.1.4H21v4h-.09a1.7 1.7 0 0 0-1.51.6Z" /></>,
  refresh: <><path d="M20 11a8 8 0 1 0-2.34 5.66" /><path d="M20 4v7h-7" /></>,
  shield: <><path d="M12 22s8-3.8 8-10V5l-8-3-8 3v7c0 6.2 8 10 8 10Z" /><path d="m9 12 2 2 4-4" /></>,
  phone: <><rect x="7" y="2" width="10" height="20" rx="3" /><path d="M11 18h2" /></>,
  monitor: <><rect x="3" y="4" width="18" height="13" rx="3" /><path d="M8 21h8M12 17v4" /></>,
  eye: <><path d="M2 12s3.5-6 10-6 10 6 10 6-3.5 6-10 6S2 12 2 12Z" /><circle cx="12" cy="12" r="2.5" /></>,
  chevron: <path d="m9 18 6-6-6-6" />,
  check: <path d="m5 12 4 4L19 6" />,
  link: <><path d="M10 13a5 5 0 0 0 7.07.07l1.42-1.42a5 5 0 0 0-7.07-7.07l-.82.82" /><path d="M14 11a5 5 0 0 0-7.07-.07L5.51 12.35a5 5 0 0 0 7.07 7.07l.82-.82" /></>,
  x: <><path d="m6 6 12 12" /><path d="m18 6-12 12" /></>
};

type IconProps = Readonly<{ name: IconName; size?: number }>;

export const Icon = memo(function Icon({ name, size = 20 }: IconProps) {
  return <svg className="icon" width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">{iconShapes[name]}</svg>;
});
