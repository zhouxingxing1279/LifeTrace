declare module "read-excel-file/browser" {
  export default function readXlsxFile(file: Blob): Promise<unknown[][]>;
}
