use crate::model::MultiRepoHistory;
use anyhow::{anyhow, Result};
use std::fs::File;
use std::path::Path;

use spsheet::ods;
use spsheet::xlsx;
use spsheet::{Book, Cell, Sheet};

pub fn generate(model: &MultiRepoHistory, output_file_path: &str) -> Result<()> {
    let path = Path::new(output_file_path);
    let extension = path.extension().and_then(|s| s.to_str());
    if extension.is_none() {
        return Err(anyhow!(
            "Couldn't derive report format from filename. Supported endings are: .csv, .ods, .xlsx"
        ));
    }

    match extension {
        Some("csv") => generate_csv(model, path),
        Some("ods") => generate_ods(model, path),
        Some("xlsx") => generate_xlsx(model, path),
        _ => Err(anyhow!(
            "Couldn't derive report format from filename. Supported endings are: .csv, .ods, .xlsx"
        )),
    }
}

trait SpreadSheetBuilder {
    fn add_cell(&mut self, cell: String) -> Result<()>;
    fn finish_row(&mut self) -> Result<()>;
}

struct CommaSeperatedSpreadsheet {
    writer: csv::Writer<File>,
}

impl CommaSeperatedSpreadsheet {
    pub fn new(output_file_path: &Path) -> Result<Self> {
        Ok(CommaSeperatedSpreadsheet {
            writer: csv::Writer::from_path(&output_file_path)?,
        })
    }

    pub fn write_to_disk(&mut self) -> Result<()> {
        Ok(self.writer.flush()?)
    }
}

impl SpreadSheetBuilder for CommaSeperatedSpreadsheet {
    fn add_cell(&mut self, cell: String) -> Result<()> {
        Ok(self.writer.write_field(cell)?)
    }

    fn finish_row(&mut self) -> Result<()> {
        Ok(self.writer.write_record(None::<&[u8]>)?)
    }
}

struct OdsXlsxSpreadsheet {
    sheet: Sheet,
    current_row: usize,
    current_column: usize,
}

impl OdsXlsxSpreadsheet {
    pub fn new() -> Result<Self> {
        Ok(OdsXlsxSpreadsheet {
            sheet: Sheet::new("oper-delta report"),
            current_row: 0,
            current_column: 0,
        })
    }
}

impl SpreadSheetBuilder for OdsXlsxSpreadsheet {
    fn add_cell(&mut self, cell: String) -> Result<()> {
        self.sheet
            .add_cell(Cell::str(cell), self.current_row, self.current_column);
        self.current_column += 1;
        Ok(())
    }

    fn finish_row(&mut self) -> Result<()> {
        self.current_row += 1;
        self.current_column = 0;
        Ok(())
    }
}

fn generate_ods(model: &MultiRepoHistory, output_file_path: &Path) -> Result<()> {
    let mut spreadsheet = OdsXlsxSpreadsheet::new()?;

    model_into_spreadsheet(&model, &mut spreadsheet)?;

    let mut book = Book::new();
    book.add_sheet(spreadsheet.sheet);
    ods::write(&book, output_file_path)
        .map_err(|e| anyhow!("Failed to write .ods file: {:?}", e))?;

    println!(
        "Wrote {} records in Open Document Format to {}",
        model.commits.len(),
        output_file_path.display()
    );
    Ok(())
}

fn generate_xlsx(model: &MultiRepoHistory, output_file_path: &Path) -> Result<()> {
    let mut spreadsheet = OdsXlsxSpreadsheet::new()?;

    model_into_spreadsheet(&model, &mut spreadsheet)?;

    let mut book = Book::new();
    book.add_sheet(spreadsheet.sheet);
    xlsx::write(&book, output_file_path)
        .map_err(|e| anyhow!("Failed to write .xlsx file: {:?}", e))?;

    println!(
        "Wrote {} records in MS Excel format to {}",
        model.commits.len(),
        output_file_path.display()
    );
    Ok(())
}

fn generate_csv(model: &MultiRepoHistory, output_file_path: &Path) -> Result<()> {
    let mut spreadsheet = CommaSeperatedSpreadsheet::new(output_file_path)?;

    model_into_spreadsheet(&model, &mut spreadsheet)?;

    spreadsheet.write_to_disk()?;

    println!(
        "Wrote {} records as comma-separated-values to {}",
        model.commits.len(),
        output_file_path.display()
    );
    Ok(())
}

fn model_into_spreadsheet(
    model: &MultiRepoHistory,
    builder: &mut dyn SpreadSheetBuilder,
) -> Result<()> {
    builder.add_cell("Commit Date".to_string())?;
    builder.add_cell("Local Path of Repo".to_string())?;
    builder.add_cell("Commit Author".to_string())?;
    builder.add_cell("Summary".to_string())?;
    builder.add_cell("Message".to_string())?;
    builder.finish_row()?;

    for commit in &model.commits {
        builder.add_cell(commit.time_as_str())?;
        builder.add_cell(commit.repo.rel_path.clone())?;
        builder.add_cell(commit.author.to_string())?;
        builder.add_cell(commit.summary.to_string())?;
        builder.add_cell(commit.message.to_string())?;
        builder.finish_row()?;
    }

    Ok(())
}
