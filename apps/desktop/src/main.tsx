import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./shared/App";
import "./shared/app.css";

const root = document.getElementById("root");
if (!root) throw new Error("Root element is missing");

createRoot(root).render(
  <StrictMode>
    <App surface="main" />
  </StrictMode>,
);
