declare module "read-excel-file" {
  export default function readXlsxFile(file: Blob): Promise<unknown[][]>;
}
