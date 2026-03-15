import { useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { writeText } from "@tauri-apps/api/clipboard";

interface ContextMenuProps {
  x: number;
  y: number;
  path: string;
  onClose: () => void;
}

export const ContextMenu = ({ x, y, path, onClose }: ContextMenuProps) => {
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClick = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("mousedown", handleClick);
    window.addEventListener("keydown", handleKey);
    return () => {
      window.removeEventListener("mousedown", handleClick);
      window.removeEventListener("keydown", handleKey);
    };
  }, [onClose]);

  // Adjust position so menu doesn't overflow the window
  const style: React.CSSProperties = {
    position: "fixed",
    top: y,
    left: x,
    zIndex: 9999,
  };

  return (
    <div
      ref={menuRef}
      style={style}
      className="bg-gray-800 border border-gray-600 rounded-md shadow-lg py-1 min-w-[180px]"
    >
      <button
        className="w-full text-left px-3 py-1.5 text-xs text-white hover:bg-gray-700 cursor-pointer"
        onClick={() => {
          invoke("show_in_folder", { path });
          onClose();
        }}
      >
        Show in Finder
      </button>
      <button
        className="w-full text-left px-3 py-1.5 text-xs text-white hover:bg-gray-700 cursor-pointer"
        onClick={() => {
          writeText(path);
          onClose();
        }}
      >
        Copy Path
      </button>
    </div>
  );
};
