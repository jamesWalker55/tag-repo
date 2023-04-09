export interface ListViewColumn {
  // what kind of column this is
  type: "path" | "name" | "tags" | "extension";
  // width of the column in pixels
  width: number;
}
