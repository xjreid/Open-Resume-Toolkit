import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./shared/App";
import { ApplicationPopup } from "./shared/ApplicationPopup";
import "./shared/app.css";

const root = document.getElementById("root");
if (!root) throw new Error("Root element is missing");

createRoot(root).render(
  <StrictMode>
    {new URLSearchParams(window.location.search).get("popup") === "1" ? (
      <ApplicationPopup />
    ) : (
      <App surface="overlay" />
    )}
  </StrictMode>,
);
