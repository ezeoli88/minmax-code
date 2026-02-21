import { useEffect, useRef } from "react";
import { useStdin } from "ink";

export function useMouseScroll(
  onScroll: (direction: "up" | "down") => void,
  active = true
) {
  const { stdin, setRawMode, isRawModeSupported } = useStdin();
  const onScrollRef = useRef(onScroll);
  onScrollRef.current = onScroll;

  useEffect(() => {
    if (!active || !isRawModeSupported || !stdin) return;

    // Enable SGR mouse reporting (wheel events)
    process.stdout.write("\x1b[?1000h"); // basic mouse reporting
    process.stdout.write("\x1b[?1006h"); // SGR extended mode

    const handleData = (data: Buffer) => {
      const chunk = data.toString("utf8");

      // SGR mouse protocol: ESC [ < MODE ; X ; Y M
      // mode 64 = scroll up, mode 65 = scroll down
      const match = chunk.match(/\x1b\[<(\d+);\d+;\d+[Mm]/);
      if (match) {
        const mode = parseInt(match[1], 10);
        if (mode === 64) onScrollRef.current("up");
        if (mode === 65) onScrollRef.current("down");
      }
    };

    stdin.on("data", handleData);

    return () => {
      stdin.off("data", handleData);
      process.stdout.write("\x1b[?1000l");
      process.stdout.write("\x1b[?1006l");
    };
  }, [stdin, isRawModeSupported, active]);
}
