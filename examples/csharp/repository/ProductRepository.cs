using System;
using System.Collections.Generic;

namespace Examples.Repositories
{
    // Identical code [Type-1/2] clone of UserRepository — identical
    // shape, renamed entity. Normalization collapses identifiers so the
    // Merkle hash matches.
    public sealed class ProductRepository
    {
        private readonly List<Product> store = new List<Product>();

        public Product? FindById(int sku)
        {
            foreach (var item in store)
            {
                if (item.Sku == sku)
                {
                    return item;
                }
            }

            return null;
        }

        public IReadOnlyList<Product> FindAll()
        {
            var copy = new List<Product>(store.Count);
            foreach (var item in store)
            {
                copy.Add(item);
            }

            return copy;
        }

        public void Insert(Product entity)
        {
            if (entity == null)
            {
                throw new ArgumentNullException(nameof(entity));
            }

            store.Add(entity);
        }

        public bool Delete(int sku)
        {
            for (int index = 0; index < store.Count; index = index + 1)
            {
                if (store[index].Sku == sku)
                {
                    store.RemoveAt(index);
                    return true;
                }
            }

            return false;
        }

        public int Count()
        {
            return store.Count;
        }
    }

    public sealed class Product
    {
        public int Sku { get; set; }
        public string Label { get; set; } = string.Empty;
    }
}
