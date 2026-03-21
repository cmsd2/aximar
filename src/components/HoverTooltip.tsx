import type { MaximaFunction } from "../types/catalog";

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
        {func.signatures[0] || func.name}
      </div>
      <div className="hover-tooltip-desc">{func.description}</div>
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
