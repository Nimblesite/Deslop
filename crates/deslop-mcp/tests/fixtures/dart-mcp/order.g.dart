// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'order.dart';

Order _$OrderFromJson(Map<String, dynamic> json) => Order(
      id: json['id'] as String,
      label: json['label'] as String,
      quantity: (json['quantity'] as num).toInt(),
      checked: json['checked'] as bool,
    );

Map<String, dynamic> _$OrderToJson(Order instance) => <String, dynamic>{
      'id': instance.id,
      'label': instance.label,
      'quantity': instance.quantity,
      'checked': instance.checked,
    };
