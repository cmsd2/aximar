import { useEffect } from "react";
import { Toolbar } from "./components/Toolbar";
import { Notebook } from "./components/Notebook";
import { useMaxima } from "./hooks/useMaxima";
import "./styles/global.css";

function App() {
  const { initSession } = useMaxima();

  useEffect(() => {
    initSession();
  }, [initSession]);

  return (
    <div className="app">
      <Toolbar />
      <Notebook />
    </div>
  );
}

export default App;
