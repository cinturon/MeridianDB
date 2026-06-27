import type { DraftCellEdit } from "./App";

interface DataGridProps {
  columns: string[];
  rows: string[][];
  editable?: boolean;
  primaryKeyColumn?: string | null;
  draftCellEdit?: DraftCellEdit | null;
  onStartEdit?: (
    rowIndex: number,
    columnName: string,
    primaryKeyValue: string,
    value: string | null,
  ) => void;
  onDraftChange?: (newValue: string) => void;
  onCancelEdit?: () => void;
  onSaveDraft?: () => void;
  saveConfirmPending?: boolean;
}

function displayCell(value: string | null): string {
  if (value === null || value === "") {
    return "—";
  }
  return value;
}

export function DataGrid({
  columns,
  rows,
  editable = false,
  primaryKeyColumn = null,
  draftCellEdit = null,
  onStartEdit,
  onDraftChange,
  onCancelEdit,
  onSaveDraft,
  saveConfirmPending = false,
}: DataGridProps) {
  const pkIndex =
    primaryKeyColumn != null ? columns.indexOf(primaryKeyColumn) : -1;

  return (
    <div className="preview-table-wrap">
      <table className="preview-table">
        <thead>
          <tr>
            {columns.map((column) => (
              <th key={column}>{column}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((row, rowIndex) => {
            const rowPkValue = pkIndex >= 0 ? row[pkIndex] ?? "" : "";

            return (
              <tr key={rowIndex}>
                {row.map((cell, cellIndex) => {
                  const columnName = columns[cellIndex];
                  const isPrimaryKeyCell = columnName === primaryKeyColumn;
                  const canEditCell =
                    editable &&
                    primaryKeyColumn != null &&
                    pkIndex >= 0 &&
                    !isPrimaryKeyCell;
                  const isActiveDraft =
                    draftCellEdit != null &&
                    draftCellEdit.targetColumn === columnName &&
                    draftCellEdit.primaryKeyValue === rowPkValue;

                  if (isActiveDraft && draftCellEdit) {
                    return (
                      <td
                        key={`${rowIndex}-${cellIndex}`}
                        className="cell-editing"
                      >
                        <div className="cell-edit-controls">
                          <input
                            type="text"
                            className="cell-edit-input"
                            value={draftCellEdit.newValue ?? ""}
                            onChange={(e) =>
                              onDraftChange?.(e.currentTarget.value)
                            }
                            aria-label={`Edit ${columnName}`}
                            disabled={saveConfirmPending}
                          />
                          <div className="cell-edit-actions">
                            <button
                              type="button"
                              className="cell-edit-save"
                              onClick={() => onSaveDraft?.()}
                              disabled={saveConfirmPending}
                            >
                              Save
                            </button>
                            <button
                              type="button"
                              className="cell-edit-cancel"
                              onClick={() => onCancelEdit?.()}
                              disabled={saveConfirmPending}
                            >
                              Cancel
                            </button>
                          </div>
                        </div>
                      </td>
                    );
                  }

                  return (
                    <td
                      key={`${rowIndex}-${cellIndex}`}
                      className={canEditCell ? "cell-editable" : undefined}
                      onClick={
                        canEditCell
                          ? () =>
                            onStartEdit?.(
                              rowIndex,
                              columnName,
                              rowPkValue,
                              cell,
                            )
                          : undefined
                      }
                      onKeyDown={
                        canEditCell
                          ? (e) => {
                            if (e.key === "Enter" || e.key === " ") {
                              e.preventDefault();
                              onStartEdit?.(
                                rowIndex,
                                columnName,
                                rowPkValue,
                                cell,
                              );
                            }
                          }
                          : undefined
                      }
                      role={canEditCell ? "button" : undefined}
                      tabIndex={canEditCell ? 0 : undefined}
                      title={
                        canEditCell
                          ? "Click to edit this cell"
                          : undefined
                      }
                    >
                      {displayCell(cell)}
                    </td>
                  );
                })}
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
