import type { MaximaFunction } from "../types/catalog";
import { MathText } from "./MathText";

interface HoverTooltipProps {
  func: MaximaFunction;
  x: number;
  y: number;
  onViewDocs: (name: string) => void;
  onMouseEnter: () => void;
  onMouseLeave: () => void;
}

export function HoverTooltip({ func, x, y, onViewDocs, onMouseEnter, onMouseLeave }: HoverTooltipProps) {
  return (
    <div
      className="hover-tooltip"
      style={{ left: x, top: y + 16 }}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
    >
      <div className="hover-tooltip-sig">
        {(func.signatures.length > 0 ? func.signatures : [func.name]).slice(0, 3).map((sig, i) => (
          <div key={i}>{sig}</div>
        ))}
        {func.signatures.length > 3 && (
          <div className="hover-tooltip-more">+{func.signatures.length - 3} more</div>
        )}
      </div>
      <div className="hover-tooltip-desc">
        <MathText text={func.description} />
      </div>
      <div className="hover-tooltip-footer">
        <span className="hover-tooltip-category">{func.category}</span>
        <button
          className="hover-tooltip-link"
          onMouseDown={(e) => {
            e.preventDefault();
            onViewDocs(func.name);
          }}
        >
          Docs &rarr;
        </button>
      </div>
    </div>
  );
}
