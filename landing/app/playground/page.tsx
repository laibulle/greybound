import type { Metadata } from "next";
import PlaygroundClient from "./PlaygroundClient";

export const metadata: Metadata = {
  title: "Greybound Playground",
  description: "Run the Greybound iced web app in the browser.",
};

export default function PlaygroundPage() {
  return <PlaygroundClient />;
}
