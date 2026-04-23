using System;
using System.Collections.Generic;

namespace Examples.Repositories
{
    // Nearly identical code [Type-3] vs UserRepository /
    // ProductRepository. Shape is the same but this variant adds a
    // cache-invalidation hook and a "soft delete" path, so the
    // structural hash differs — LSH + embeddings still surface the
    // family resemblance.
    public sealed class OrderRepository
    {
        private readonly List<Order> store = new List<Order>();
        private readonly HashSet<int> deletedIds = new HashSet<int>();

        public Order? FindById(int orderNumber)
        {
            if (deletedIds.Contains(orderNumber))
            {
                return null;
            }

            foreach (var item in store)
            {
                if (item.OrderNumber == orderNumber)
                {
                    return item;
                }
            }

            return null;
        }

        public IReadOnlyList<Order> FindAll()
        {
            var copy = new List<Order>(store.Count);
            foreach (var item in store)
            {
                if (!deletedIds.Contains(item.OrderNumber))
                {
                    copy.Add(item);
                }
            }

            return copy;
        }

        public void Insert(Order entity)
        {
            if (entity == null)
            {
                throw new ArgumentNullException(nameof(entity));
            }

            deletedIds.Remove(entity.OrderNumber);
            store.Add(entity);
        }

        public bool Delete(int orderNumber)
        {
            for (int index = 0; index < store.Count; index = index + 1)
            {
                if (store[index].OrderNumber == orderNumber)
                {
                    deletedIds.Add(orderNumber);
                    return true;
                }
            }

            return false;
        }

        public int Count()
        {
            return store.Count - deletedIds.Count;
        }
    }

    public sealed class Order
    {
        public int OrderNumber { get; set; }
        public decimal Total { get; set; }
    }
}
