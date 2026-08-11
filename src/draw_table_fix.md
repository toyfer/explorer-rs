## Fix applied in next commit via search-replace of draw_table

Replace in draw_table:
```
let scroll_to = self.scroll_to_row.take();

egui_extras::TableBuilder::new(ui)
```
with:
```
let scroll_to = self.scroll_to_row.take();
let egui_ctx = ui.ctx().clone();

egui_extras::TableBuilder::new(ui)
```

Replace:
```
let tex = self.get_or_load_icon(row.ctx(), &entry.path, entry.is_dir);
```
with:
```
let tex = self.get_or_load_icon(&egui_ctx, &entry.path, entry.is_dir);
```

Replace Stroke 1.0 with 1.0_f32
Replace `let mut addr_edit` with `let addr_edit`
