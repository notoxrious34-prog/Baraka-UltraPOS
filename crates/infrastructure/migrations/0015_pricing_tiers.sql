-- Migration 0015: Customer Pricing Tiers & Unit Wholesale Price
ALTER TABLE customers ADD COLUMN customer_tier TEXT NOT NULL DEFAULT 'retail';
ALTER TABLE product_units ADD COLUMN wholesale_price_centimes INTEGER;
