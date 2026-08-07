declare module "read-excel-file/browser" {
  export default function readXlsxFile(file: Blob): Promise<unknown[][]>;
  export function readSheet(file: Blob): Promise<unknown[][]>;
}
