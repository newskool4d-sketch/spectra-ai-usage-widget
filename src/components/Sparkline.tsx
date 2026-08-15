import { memo, useId } from "react";

type SparklineProps = Readonly<{ values: readonly number[]; color: string; width?: number; height?: number }>;

export const Sparkline = memo(function Sparkline({ values, color, width = 160, height = 54 }: SparklineProps) {
  const id = useId().replaceAll(":", "");
  const min = Math.min(...values) - 5;
  const max = Math.max(...values) + 5;
  const points = values.map((value, index) => {
    const x = (index / (values.length - 1)) * width;
    const y = height - ((value - min) / (max - min)) * height;
    return `${x.toFixed(1)},${y.toFixed(1)}`;
  }).join(" ");
  const lastY = points.split(" ").at(-1)?.split(",")[1] ?? "0";
  const fillPoints = `0,${height} ${points} ${width},${height}`;

  return <svg className="sparkline" viewBox={`0 0 ${width} ${height}`} role="img" aria-label="7일 사용 추세">
    <defs><linearGradient id={`g-${id}`} x1="0" y1="0" x2="0" y2="1"><stop offset="0" stopColor={color} stopOpacity=".36" /><stop offset="1" stopColor={color} stopOpacity="0" /></linearGradient></defs>
    <polygon points={fillPoints} fill={`url(#g-${id})`} />
    <polyline points={points} fill="none" stroke={color} strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" />
    <circle cx={width} cy={lastY} r="3.5" fill={color} />
  </svg>;
});
