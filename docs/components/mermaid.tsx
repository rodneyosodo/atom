'use client';

import { useEffect, useId, useRef, useState } from 'react';

type MermaidProps = {
  chart: string;
};

export function Mermaid({ chart }: MermaidProps) {
  const id = `mermaid-${useId().replace(/:/g, '')}`;
  const mounted = useRef(true);
  const [svg, setSvg] = useState('');
  const [error, setError] = useState('');

  useEffect(() => {
    mounted.current = true;
    setError('');

    async function render() {
      try {
        const { default: mermaid } = await import('mermaid');
        mermaid.initialize({
          startOnLoad: false,
          securityLevel: 'strict',
          theme: 'neutral',
        });
        const result = await mermaid.render(id, chart);
        if (mounted.current) {
          setSvg(result.svg);
        }
      } catch (err) {
        if (mounted.current) {
          setError(err instanceof Error ? err.message : 'Diagram failed to render');
        }
      }
    }

    render();

    return () => {
      mounted.current = false;
    };
  }, [chart, id]);

  if (error) {
    return (
      <pre className="overflow-auto rounded-lg border bg-muted p-4 text-sm text-muted-foreground">
        {chart}
      </pre>
    );
  }

  return (
    <div
      className="my-6 overflow-x-auto rounded-lg border bg-background p-4"
      dangerouslySetInnerHTML={{ __html: svg }}
    />
  );
}
