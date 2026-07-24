import React from "react";

interface ErrorBoundaryProps {
  children: React.ReactNode;
  /**
   * Rendered when a descendant throws during render/lifecycle. Receives the
   * error and a `reset` callback that clears the error so the subtree can try
   * to render again (e.g. after new props arrive).
   */
  fallback?: (error: Error, reset: () => void) => React.ReactNode;
  /** Optional label included in the console error for easier debugging. */
  label?: string;
}

interface ErrorBoundaryState {
  error: Error | null;
}

/**
 * Generic error boundary. Catches synchronous render/lifecycle errors in the
 * wrapped subtree and shows a fallback instead of letting the error propagate
 * to the root (which would blank the entire app). Async errors — thrown from
 * promises, event handlers, or requestAnimationFrame — are NOT caught by React
 * error boundaries; those must be handled at their call site (see PlotlyChart).
 */
export class ErrorBoundary extends React.Component<
  ErrorBoundaryProps,
  ErrorBoundaryState
> {
  state: ErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    console.error(
      `[ErrorBoundary${this.props.label ? `: ${this.props.label}` : ""}]`,
      error,
      info.componentStack,
    );
  }

  reset = () => this.setState({ error: null });

  render() {
    if (this.state.error) {
      if (this.props.fallback) {
        return this.props.fallback(this.state.error, this.reset);
      }
      return (
        <pre style={{ padding: 20, color: "red", whiteSpace: "pre-wrap" }}>
          {this.state.error.message}
          {"\n\n"}
          {this.state.error.stack}
        </pre>
      );
    }
    return this.props.children;
  }
}
