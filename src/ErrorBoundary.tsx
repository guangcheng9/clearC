import { Component, ErrorInfo, ReactNode } from "react";

type ErrorBoundaryProps = {
  children: ReactNode;
};

type ErrorBoundaryState = {
  error: Error | null;
};

export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = {
    error: null,
  };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("ClearC render error", error, info);
  }

  render() {
    if (this.state.error) {
      return (
        <div className="fatal-panel">
          <h2>页面渲染异常</h2>
          <p>{this.state.error.message}</p>
          <button
            className="inline-action"
            onClick={() => this.setState({ error: null })}
            type="button"
          >
            重新显示页面
          </button>
        </div>
      );
    }

    return this.props.children;
  }
}
