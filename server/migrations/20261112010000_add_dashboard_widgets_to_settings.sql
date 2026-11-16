-- Dashboard: the user's widgets in display order, each shown or hidden, e.g.
-- [{"id": "transactions", "visible": true}]. A widget the list doesn't mention shows, after the
-- listed ones, so an empty list is every widget shown in the default order.
ALTER TABLE settings ADD COLUMN dashboard_widgets JSONB NOT NULL DEFAULT '[]';
