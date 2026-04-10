interface ProgressBarProps {
  value: number;
  max?: number;
  label?: string;
  showPercentage?: boolean;
  indeterminate?: boolean;
  className?: string;
}

export default function ProgressBar({
  value,
  max = 100,
  label,
  showPercentage = true,
  indeterminate = false,
  className = "",
}: ProgressBarProps) {
  const percentage = Math.min(Math.round((value / max) * 100), 100);

  return (
    <div className={`w-full ${className}`}>
      {(label || showPercentage) && (
        <div className="flex items-center justify-between mb-1.5">
          {label && (
            <span className="text-sm text-[var(--color-text-secondary)]">
              {label}
            </span>
          )}
          {showPercentage && !indeterminate && (
            <span className="text-sm font-medium text-[var(--color-text-primary)]">
              {percentage}%
            </span>
          )}
        </div>
      )}
      <div className="w-full h-2 rounded-full bg-[var(--color-bg-input)] overflow-hidden">
        {indeterminate ? (
          <div
            className="h-full rounded-full bg-[var(--color-accent-primary)] animate-[indeterminate_1.5s_ease-in-out_infinite]"
            style={{ width: "40%" }}
          />
        ) : (
          <div
            className="h-full rounded-full bg-[var(--color-accent-primary)] transition-[width] duration-300 ease-out"
            style={{ width: `${percentage}%` }}
          />
        )}
      </div>
      <style>{`
        @keyframes indeterminate {
          0% { transform: translateX(-100%); }
          100% { transform: translateX(350%); }
        }
      `}</style>
    </div>
  );
}
