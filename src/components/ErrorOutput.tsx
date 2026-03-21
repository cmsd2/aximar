interface ErrorOutputProps {
  error: string;
}

export function ErrorOutput({ error }: ErrorOutputProps) {
  return (
    <div className="error-output">
      <pre>{error}</pre>
    </div>
  );
}
