import { state } from "./state";

export function setQuery(query: string) {
  console.log("Set query to:", query);
  state.query = query;
}
